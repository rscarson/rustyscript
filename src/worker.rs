//! Provides a worker thread that can be used to run javascript code in a separate thread through a channel pair
//! It also provides a default worker implementation that can be used without any additional setup:
//! ```rust
//! use rustyscript::{Error, worker::{Worker, DefaultWorker, DefaultWorkerOptions}};
//! use std::time::Duration;
//!
//! fn main() -> Result<(), Error> {
//!     let worker = DefaultWorker::new(DefaultWorkerOptions {
//!         default_entrypoint: None,
//!         timeout: Duration::from_secs(5),
//!     })?;
//!
//!     worker.register_function("add".to_string(), |args, _state| {
//!         let a = args[0].as_i64().unwrap();
//!         let b = args[1].as_i64().unwrap();
//!         let result = a + b;
//!         Ok(result.into())
//!     })?;
//!     let result: i32 = worker.eval("add(5, 5)".to_string())?;
//!     assert_eq!(result, 10);
//!     Ok(())
//! }

use crate::Error;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};

/// A worker thread that can be used to run javascript code in a separate thread
/// Contains a channel pair for communication, and a single runtime instance
///
/// This worker is generic over an implementation of the [worker::InnerWorker] trait
/// This allows flexibility in the runtime used by the worker, as well as the types of queries and responses that can be used
///
/// For a simple worker that uses the default runtime, see [worker::DefaultWorker]
pub struct Worker<W>
where
    W: InnerWorker,
{
    handle: JoinHandle<()>,
    tx: Sender<W::Query>,
    rx: Receiver<W::Response>,
}

impl<W> Worker<W>
where
    W: InnerWorker,
{
    /// Create a new worker instance
    pub fn new(options: W::RuntimeOptions) -> Result<Self, Error> {
        let (qtx, qrx) = channel();
        let (rtx, rrx) = channel();
        let (init_tx, init_rx) = channel::<Option<Error>>();

        let handle = spawn(move || {
            let rx = qrx;
            let tx = rtx;
            let itx = init_tx;

            let runtime = match W::init_runtime(options) {
                Ok(rt) => rt,
                Err(e) => {
                    itx.send(Some(e)).unwrap();
                    return;
                }
            };

            itx.send(None).unwrap();
            W::thread(runtime, rx, tx);
        });

        let worker = Self {
            handle,
            tx: qtx,
            rx: rrx,
        };

        // Wait for initialization to complete
        match init_rx.recv() {
            Ok(None) => Ok(worker),

            // Initialization failed
            Ok(Some(e)) => Err(e),

            // Parser crashed on startup
            _ => {
                // This can be replaced with `?` by calling `try_new` on the deno_core::Runtime once that change makes it into a release
                let e = worker
                    .handle
                    .join()
                    .err()
                    .and_then(|e| {
                        e.downcast_ref::<String>()
                            .cloned()
                            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    })
                    .unwrap_or_else(|| "Could not start runtime thread".to_string());

                // Remove everything after the words 'Stack backtrace'
                let e = match e.split("Stack backtrace").next() {
                    Some(e) => e.trim(),
                    None => &e,
                }
                .to_string();

                Err(Error::Runtime(e))
            }
        }
    }

    /// Send a request to the worker
    /// This will not block the current thread
    /// Will return an error if the worker has stopped or panicked
    pub fn send(&self, query: W::Query) -> Result<(), Error> {
        self.tx
            .send(query)
            .map_err(|e| Error::Runtime(e.to_string()))
    }

    /// Receive a response from the worker
    /// This will block the current thread until a response is received
    /// Will return an error if the worker has stopped or panicked
    pub fn receive(&self) -> Result<W::Response, Error> {
        self.rx.recv().map_err(|e| Error::Runtime(e.to_string()))
    }

    /// Send a request to the worker and wait for a response
    /// This will block the current thread until a response is received
    /// Will return an error if the worker has stopped or panicked
    pub fn send_and_await(&self, query: W::Query) -> Result<W::Response, Error> {
        self.send(query)?;
        self.receive()
    }

    /// Consume the worker and wait for the thread to finish
    /// WARNING: This will block the current thread until the worker has finished
    ///          Make sure to send a stop message to the worker before calling this!
    pub fn join(self) -> Result<(), Error> {
        self.handle
            .join()
            .map_err(|_| Error::Runtime("Worker thread panicked".to_string()))
    }
}

/// An implementation of the worker trait for a specific runtime
/// This allows flexibility in the runtime used by the worker
/// As well as the types of queries and responses that can be used
///
/// Implement this trait for a specific runtime to use it with the worker
/// For an example implementation, see [worker::DefaultWorker]
pub trait InnerWorker
where
    Self: Send,
    <Self as InnerWorker>::RuntimeOptions: std::marker::Send + 'static,
    <Self as InnerWorker>::Query: std::marker::Send + 'static,
    <Self as InnerWorker>::Response: std::marker::Send + 'static,
{
    /// The type of runtime used by this worker
    /// This can just be `rustyscript::Runtime` if you don't need to use a custom runtime
    type Runtime;

    /// The type of options that can be used to initialize the runtime
    /// Cannot be `rustyscript::RuntimeOptions` because it is not `Send`
    type RuntimeOptions;

    /// The type of query that can be sent to the worker
    /// This should be an enum that contains all possible queries
    type Query;

    /// The type of response that can be received from the worker
    /// This should be an enum that contains all possible responses
    type Response;

    /// Initialize the runtime used by the worker
    /// This should return a new instance of the runtime that will respond to queries
    fn init_runtime(options: Self::RuntimeOptions) -> Result<Self::Runtime, Error>;

    /// Handle a query sent to the worker
    /// Must always return a response of some kind
    fn handle_query(runtime: &mut Self::Runtime, query: Self::Query) -> Self::Response;

    /// The main thread function that will be run by the worker
    /// This should handle all incoming queries and send responses back
    fn thread(mut runtime: Self::Runtime, rx: Receiver<Self::Query>, tx: Sender<Self::Response>) {
        loop {
            let msg = match rx.recv() {
                Ok(msg) => msg,
                Err(_) => break,
            };

            let response = Self::handle_query(&mut runtime, msg);
            tx.send(response).unwrap();
        }
    }
}

/// A worker implementation that uses the default runtime
/// This is the simplest way to use the worker, as it requires no additional setup
/// It attempts to provide as much functionality as possible from the standard runtime
///
/// Please note that it uses serde_json::Value for queries and responses, which comes with a performance cost
/// For a more performant worker, or to use extensions and/or loader caches, you'll need to implement your own worker
pub struct DefaultWorker(Worker<DefaultWorker>);
impl InnerWorker for DefaultWorker {
    type Runtime = (
        crate::Runtime,
        std::collections::HashMap<deno_core::ModuleId, crate::ModuleHandle>,
    );
    type RuntimeOptions = DefaultWorkerOptions;
    type Query = DefaultWorkerQuery;
    type Response = DefaultWorkerResponse;

    fn init_runtime(options: Self::RuntimeOptions) -> Result<Self::Runtime, Error> {
        let runtime = crate::Runtime::new(crate::RuntimeOptions {
            default_entrypoint: options.default_entrypoint,
            timeout: options.timeout,
            ..Default::default()
        })?;
        let modules = std::collections::HashMap::new();
        Ok((runtime, modules))
    }

    fn handle_query(runtime: &mut Self::Runtime, query: Self::Query) -> Self::Response {
        let (runtime, modules) = runtime;
        match query {
            DefaultWorkerQuery::Stop => Self::Response::Ok(()),

            DefaultWorkerQuery::RegisterFunction(name, func) => {
                match runtime.register_function(&name, func) {
                    Ok(_) => Self::Response::Ok(()),
                    Err(e) => Self::Response::Error(e),
                }
            }

            DefaultWorkerQuery::Eval(code) => match runtime.eval(&code) {
                Ok(v) => Self::Response::Value(v),
                Err(e) => Self::Response::Error(e),
            },

            DefaultWorkerQuery::LoadMainModule(module) => match runtime.load_module(&module) {
                Ok(handle) => {
                    let id = handle.id();
                    modules.insert(id, handle);
                    Self::Response::ModuleId(id)
                }
                Err(e) => Self::Response::Error(e),
            },

            DefaultWorkerQuery::LoadModule(module) => match runtime.load_module(&module) {
                Ok(handle) => {
                    let id = handle.id();
                    modules.insert(id, handle);
                    Self::Response::ModuleId(id)
                }
                Err(e) => Self::Response::Error(e),
            },

            DefaultWorkerQuery::CallEntrypoint(id, args) => match modules.get(&id) {
                Some(handle) => match runtime.call_entrypoint(handle, &args) {
                    Ok(v) => Self::Response::Value(v),
                    Err(e) => Self::Response::Error(e),
                },
                None => Self::Response::Error(Error::Runtime("Module not found".to_string())),
            },

            DefaultWorkerQuery::CallFunction(id, name, args) => {
                let handle = if let Some(id) = id {
                    match modules.get(&id) {
                        Some(handle) => Some(handle),
                        None => {
                            return Self::Response::Error(Error::Runtime(
                                "Module not found".to_string(),
                            ))
                        }
                    }
                } else {
                    None
                };

                match runtime.call_function(handle, &name, &args) {
                    Ok(v) => Self::Response::Value(v),
                    Err(e) => Self::Response::Error(e),
                }
            }

            DefaultWorkerQuery::GetValue(id, name) => {
                let handle = if let Some(id) = id {
                    match modules.get(&id) {
                        Some(handle) => Some(handle),
                        None => {
                            return Self::Response::Error(Error::Runtime(
                                "Module not found".to_string(),
                            ))
                        }
                    }
                } else {
                    None
                };

                match runtime.get_value(handle, &name) {
                    Ok(v) => Self::Response::Value(v),
                    Err(e) => Self::Response::Error(e),
                }
            }
        }
    }

    // Custom thread impl to handle stop
    fn thread(mut runtime: Self::Runtime, rx: Receiver<Self::Query>, tx: Sender<Self::Response>) {
        loop {
            let msg = match rx.recv() {
                Ok(msg) => msg,
                Err(_) => break,
            };

            match &msg {
                DefaultWorkerQuery::Stop => {
                    tx.send(Self::Response::Ok(())).unwrap();
                    break;
                }
                _ => {
                    let response = Self::handle_query(&mut runtime, msg);
                    tx.send(response).unwrap();
                }
            }
        }
    }
}
impl DefaultWorker {
    /// Create a new worker instance
    pub fn new(options: DefaultWorkerOptions) -> Result<Self, Error> {
        Worker::new(options).map(Self)
    }

    /// Stop the worker and wait for it to finish
    /// Consumes the worker and returns an error if the worker panicked
    pub fn stop(self) -> Result<(), Error> {
        self.0.send(DefaultWorkerQuery::Stop)?;
        self.0.join()
    }

    /// Register a rust function with the worker
    /// This function will be callable from javascript
    pub fn register_function(&self, name: String, func: crate::RsFunction) -> Result<(), Error> {
        self.0
            .send_and_await(DefaultWorkerQuery::RegisterFunction(name, func))?;
        Ok(())
    }

    /// Evaluate a string of javascript code
    /// Returns the result of the evaluation
    pub fn eval<T>(&self, code: String) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        match self.0.send_and_await(DefaultWorkerQuery::Eval(code))? {
            DefaultWorkerResponse::Value(v) => Ok(crate::serde_json::from_value(v)?),
            DefaultWorkerResponse::Error(e) => Err(e),
            _ => Err(Error::Runtime(
                "Unexpected response from the worker".to_string(),
            )),
        }
    }

    /// Load a module into the worker as the main module
    /// Returns the module id of the loaded module
    pub fn load_main_module(&self, module: crate::Module) -> Result<deno_core::ModuleId, Error> {
        match self
            .0
            .send_and_await(DefaultWorkerQuery::LoadMainModule(module))?
        {
            DefaultWorkerResponse::ModuleId(id) => Ok(id),
            DefaultWorkerResponse::Error(e) => Err(e),
            _ => Err(Error::Runtime(
                "Unexpected response from the worker".to_string(),
            )),
        }
    }

    /// Load a module into the worker as a side module
    /// Returns the module id of the loaded module
    pub fn load_module(&self, module: crate::Module) -> Result<deno_core::ModuleId, Error> {
        match self
            .0
            .send_and_await(DefaultWorkerQuery::LoadModule(module))?
        {
            DefaultWorkerResponse::ModuleId(id) => Ok(id),
            DefaultWorkerResponse::Error(e) => Err(e),
            _ => Err(Error::Runtime(
                "Unexpected response from the worker".to_string(),
            )),
        }
    }

    /// Call the entrypoint function in a module
    /// Returns the result of the function call
    /// The module id must be the id of a module loaded with `load_main_module` or `load_module`
    pub fn call_entrypoint<T>(
        &self,
        id: deno_core::ModuleId,
        args: Vec<crate::serde_json::Value>,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        match self
            .0
            .send_and_await(DefaultWorkerQuery::CallEntrypoint(id, args))?
        {
            DefaultWorkerResponse::Value(v) => {
                crate::serde_json::from_value(v).map_err(Error::from)
            }
            DefaultWorkerResponse::Error(e) => Err(e),
            _ => Err(Error::Runtime(
                "Unexpected response from the worker".to_string(),
            )),
        }
    }

    /// Call a function in a module
    /// Returns the result of the function call
    /// The module id must be the id of a module loaded with `load_main_module` or `load_module`
    pub fn call_function<T>(
        &self,
        module_context: Option<deno_core::ModuleId>,
        name: String,
        args: Vec<crate::serde_json::Value>,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        match self
            .0
            .send_and_await(DefaultWorkerQuery::CallFunction(module_context, name, args))?
        {
            DefaultWorkerResponse::Value(v) => {
                crate::serde_json::from_value(v).map_err(Error::from)
            }
            DefaultWorkerResponse::Error(e) => Err(e),
            _ => Err(Error::Runtime(
                "Unexpected response from the worker".to_string(),
            )),
        }
    }

    /// Get a value from a module
    /// The module id must be the id of a module loaded with `load_main_module` or `load_module`
    pub fn get_value<T>(
        &self,
        module_context: Option<deno_core::ModuleId>,
        name: String,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        match self
            .0
            .send_and_await(DefaultWorkerQuery::GetValue(module_context, name))?
        {
            DefaultWorkerResponse::Value(v) => {
                crate::serde_json::from_value(v).map_err(Error::from)
            }
            DefaultWorkerResponse::Error(e) => Err(e),
            _ => Err(Error::Runtime(
                "Unexpected response from the worker".to_string(),
            )),
        }
    }
}

/// Options for the default worker
#[derive(Default, Clone)]
pub struct DefaultWorkerOptions {
    /// The default entrypoint function to use if none is registered
    pub default_entrypoint: Option<String>,

    /// The timeout to use for the runtime
    pub timeout: std::time::Duration,
}

/// Query types for the default worker
pub enum DefaultWorkerQuery {
    /// Stops the worker
    Stop,

    /// Registers a function with the worker
    RegisterFunction(String, crate::RsFunction),

    /// Evaluates a string of javascript code
    Eval(String),

    /// Loads a module into the worker as the main module
    LoadMainModule(crate::Module),

    /// Loads a module into the worker as a side module
    LoadModule(crate::Module),

    /// Calls an entrypoint function in a module
    CallEntrypoint(deno_core::ModuleId, Vec<crate::serde_json::Value>),

    /// Calls a function in a module
    CallFunction(
        Option<deno_core::ModuleId>,
        String,
        Vec<crate::serde_json::Value>,
    ),

    /// Gets a value from a module
    GetValue(Option<deno_core::ModuleId>, String),
}

/// Response types for the default worker
pub enum DefaultWorkerResponse {
    /// A successful response with a value
    Value(crate::serde_json::Value),

    /// A successful response with a module id
    ModuleId(deno_core::ModuleId),

    /// A successful response with no value
    Ok(()),

    /// An error response
    Error(Error),
}
