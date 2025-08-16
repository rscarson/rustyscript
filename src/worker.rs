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
//!         ..Default::default()
//!     })?;
//!
//!     let result: i32 = worker.eval("5 + 5".to_string())?;
//!     assert_eq!(result, 10);
//!     Ok(())
//! }
use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc::{channel, Receiver, Sender},
    thread::{spawn, JoinHandle},
};

use crate::{Error, RuntimeOptions};

/// A pool of worker threads that can be used to run javascript code in parallel
/// Uses a round-robin strategy to distribute work between workers
/// Each worker is an independent runtime instance
pub struct WorkerPool<W>
where
    W: InnerWorker,
{
    workers: Vec<Rc<RefCell<Worker<W>>>>,
    next_worker: usize,
    options: W::RuntimeOptions,
}

impl<W> WorkerPool<W>
where
    W: InnerWorker,
{
    /// Create a new worker pool with the specified number of workers
    ///
    /// # Errors
    /// Can fail if a runtime cannot be initialized (usually due to extension issues)
    pub fn new(options: W::RuntimeOptions, n_workers: u32) -> Result<Self, Error> {
        crate::init_platform(n_workers, true);
        let mut workers = Vec::with_capacity(n_workers as usize + 1);
        for _ in 0..n_workers {
            workers.push(Rc::new(RefCell::new(Worker::new(options.clone())?)));
        }

        Ok(Self {
            workers,
            next_worker: 0,
            options,
        })
    }

    /// Returns the runtime options used by the workers in the pool
    #[must_use]
    pub fn options(&self) -> &W::RuntimeOptions {
        &self.options
    }

    /// Stop all workers in the pool and wait for them to finish
    pub fn shutdown(self) {
        for worker in self.workers {
            worker.borrow_mut().shutdown();
        }
    }

    /// Get the number of workers in the pool
    #[must_use]
    pub fn len(&self) -> usize {
        self.workers.len()
    }

    /// Check if the pool is empty
    /// This will be true if the pool has no workers
    /// This can happen if the pool was created with 0 workers
    /// Which is not particularly useful, but is allowed
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.workers.is_empty()
    }

    /// Get a worker by its index in the pool
    #[must_use]
    pub fn worker_by_id(&self, id: usize) -> Option<Rc<RefCell<Worker<W>>>> {
        Some(Rc::clone(self.workers.get(id)?))
    }

    /// Get the next worker in the pool
    pub fn next_worker(&mut self) -> Rc<RefCell<Worker<W>>> {
        let worker = &self.workers[self.next_worker];
        self.next_worker = (self.next_worker + 1) % self.workers.len();
        Rc::clone(worker)
    }

    /// Send a request to the next worker in the pool
    /// This will block the current thread until the response is received
    ///
    /// # Errors
    /// Will return an error if the worker has already been stopped, or if the worker thread panicked
    pub fn send_and_await(&mut self, query: W::Query) -> Result<W::Response, Error> {
        self.next_worker().borrow().send_and_await(query)
    }

    /// Evaluate a string of non-ecma javascript code in a separate thread
    /// The code is evaluated in a new runtime instance, which is then destroyed
    /// Returns a handle to the thread that is running the code
    #[must_use = "The returned thread handle will return a Result<T, Error> when joined"]
    pub fn eval_in_thread<T>(code: String) -> std::thread::JoinHandle<Result<T, Error>>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        deno_core::JsRuntime::init_platform(None, true);
        std::thread::spawn(move || {
            let mut runtime = crate::Runtime::new(RuntimeOptions::default())?;
            runtime.eval(&code)
        })
    }
}

/// A worker thread that can be used to run javascript code in a separate thread
/// Contains a channel pair for communication, and a single runtime instance
///
/// This worker is generic over an implementation of the [`InnerWorker`] trait
/// This allows flexibility in the runtime used by the worker, as well as the types of queries and responses that can be used
///
/// For a simple worker that uses the default runtime, see [`DefaultWorker`]
pub struct Worker<W>
where
    W: InnerWorker,
{
    handle: Option<JoinHandle<()>>,
    tx: Option<Sender<W::Query>>,
    rx: Receiver<W::Response>,
}

impl<W> Worker<W>
where
    W: InnerWorker,
{
    /// Create a new worker instance
    ///
    /// # Errors
    /// Can fail if the runtime cannot be initialized (usually due to extension issues)
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
                    itx.send(Some(e)).ok(); // Stopping anyway, so no need to check for errors
                    return;
                }
            };

            if itx.send(None).is_ok() {
                W::thread(runtime, rx, tx);
            }
        });

        let worker = Self {
            handle: Some(handle),
            tx: Some(qtx),
            rx: rrx,
        };

        // Wait for initialization to complete
        match init_rx.recv() {
            Ok(None) => Ok(worker),

            // Initialization failed
            Ok(Some(e)) => Err(e),

            // Parser crashed on startup
            _ => {
                let Some(handle) = worker.handle else {
                    return Err(Error::Runtime(
                        "Could not start runtime thread: Worker handle missing".to_string(),
                    ));
                };

                // Attempt to join the thread to get the error message
                let Err(e) = handle.join() else {
                    return Err(Error::Runtime("Could not start runtime thread".to_string()));
                };

                // Get the actual error message - String, &str, or default message
                let e = if let Some(e) = e.downcast_ref::<String>() {
                    e.clone()
                } else if let Some(e) = e.downcast_ref::<&str>() {
                    (*e).to_string()
                } else {
                    "Could not start runtime thread".to_string()
                };

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

    /// Stop the worker and wait for it to finish
    /// Stops by destroying the sender, which will cause the thread to exit the loop and finish
    ///
    /// WARNING: If implementing a custom `thread` function, make sure to handle rx failures gracefully
    ///          Otherwise this will block indefinitely
    pub fn shutdown(&mut self) {
        if let (Some(tx), Some(hnd)) = (self.tx.take(), self.handle.take()) {
            // We can stop the thread by destroying the sender
            // This will cause the thread to exit the loop and finish
            drop(tx);
            hnd.join().ok();
        }
    }

    /// Send a request to the worker
    /// This will not block the current thread
    ///
    /// # Errors
    /// Will return an error if the worker has already been stopped, or if the worker thread panicked
    pub fn send(&self, query: W::Query) -> Result<(), Error> {
        match &self.tx {
            None => return Err(Error::WorkerHasStopped),
            Some(tx) => tx,
        }
        .send(query)
        .map_err(|e| Error::Runtime(e.to_string()))
    }

    /// Receive a response from the worker
    /// This will block the current thread until a response is received
    ///
    /// # Errors
    /// Will return an error if the worker has already been stopped, or if the worker thread panicked
    pub fn receive(&self) -> Result<W::Response, Error> {
        self.rx.recv().map_err(|e| Error::Runtime(e.to_string()))
    }

    /// Try to receive a response from the worker without blocking
    /// This will return `Ok(None)` if no response is available
    ///
    /// # Errors
    /// Will return an error if the worker has already been stopped, or if the worker thread panicked
    pub fn try_receive(&self) -> Result<Option<W::Response>, Error> {
        match self.rx.try_recv() {
            Ok(v) => Ok(Some(v)),
            Err(e) => match e {
                std::sync::mpsc::TryRecvError::Empty => Ok(None),
                std::sync::mpsc::TryRecvError::Disconnected => Err(Error::Runtime(e.to_string())),
            },
        }
    }

    /// Send a request to the worker and wait for a response
    /// This will block the current thread until a response is received
    /// Will return an error if the worker has stopped or panicked
    ///
    /// # Errors
    /// Will return an error if the worker has already been stopped, or if the worker thread panicked
    pub fn send_and_await(&self, query: W::Query) -> Result<W::Response, Error> {
        self.send(query)?;
        self.receive()
    }

    /// Consume the worker and wait for the thread to finish
    ///
    /// WARNING: If implementing a custom `thread` function, make sure to handle rx failures gracefully
    ///          Otherwise this will block indefinitely
    ///
    /// # Errors
    /// Will return an error if the worker has already been stopped, or if the worker thread panicked
    pub fn join(mut self) -> Result<(), Error> {
        self.shutdown();
        match self.handle {
            Some(hnd) => hnd
                .join()
                .map_err(|_| Error::Runtime("Worker thread panicked".to_string())),
            None => Ok(()),
        }
    }
}

/// An implementation of the worker trait for a specific runtime
/// This allows flexibility in the runtime used by the worker
/// As well as the types of queries and responses that can be used
///
/// Implement this trait for a specific runtime to use it with the worker
/// For an example implementation, see [`DefaultWorker`]
pub trait InnerWorker
where
    Self: Send,
    <Self as InnerWorker>::RuntimeOptions: std::marker::Send + 'static + Clone,
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
    ///
    /// # Errors
    /// Can fail if the runtime cannot be initialized (usually due to extension issues)
    fn init_runtime(options: Self::RuntimeOptions) -> Result<Self::Runtime, Error>;

    /// Handle a query sent to the worker
    /// Must always return a response of some kind
    fn handle_query(runtime: &mut Self::Runtime, query: Self::Query) -> Self::Response;

    /// The main thread function that will be run by the worker
    /// This should handle all incoming queries and send responses back
    fn thread(mut runtime: Self::Runtime, rx: Receiver<Self::Query>, tx: Sender<Self::Response>) {
        loop {
            let Ok(msg) = rx.recv() else {
                break;
            };

            let response = Self::handle_query(&mut runtime, msg);
            if tx.send(response).is_err() {
                break;
            }
        }
    }
}

/// A worker implementation that uses the default runtime
/// This is the simplest way to use the worker, as it requires no additional setup
/// It attempts to provide as much functionality as possible from the standard runtime
///
/// Please note that it uses `serde_json::Value` for queries and responses, which comes with a performance cost
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
            shared_array_buffer_store: options.shared_array_buffer_store,
            startup_snapshot: options.startup_snapshot,
            ..Default::default()
        })?;
        let modules = std::collections::HashMap::new();
        Ok((runtime, modules))
    }

    fn handle_query(runtime: &mut Self::Runtime, query: Self::Query) -> Self::Response {
        let (runtime, modules) = runtime;
        match query {
            DefaultWorkerQuery::Eval(code) => match runtime.eval(&code) {
                Ok(v) => Self::Response::Value(v),
                Err(e) => Self::Response::Error(e),
            },

            DefaultWorkerQuery::LoadMainModule(module) => {
                match runtime.load_modules(&module, vec![]) {
                    Ok(handle) => {
                        let id = handle.id();
                        modules.insert(id, handle);
                        Self::Response::ModuleId(id)
                    }
                    Err(e) => Self::Response::Error(e),
                }
            }

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
}
impl DefaultWorker {
    /// Create a new worker instance
    ///
    /// # Errors
    /// Can fail if the runtime cannot be initialized (usually due to extension issues)
    pub fn new(options: DefaultWorkerOptions) -> Result<Self, Error> {
        Worker::new(options).map(Self)
    }

    /// Get a reference to the underlying worker instance
    #[must_use]
    pub fn as_worker(&self) -> &Worker<DefaultWorker> {
        &self.0
    }

    /// Evaluate a string of javascript code
    /// Returns the result of the evaluation
    ///
    /// # Errors
    /// Can fail a runtime error occurs during evaluation, or if the return value cannot be deserialized into the requested type
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
    ///
    /// # Errors
    /// Can fail if execution of the module fails
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
    ///
    /// # Errors
    /// Can fail if execution of the module fails
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
    ///
    /// # Errors
    /// Can fail the module is not found, if there is no entrypoint function, if the entrypoint function returns an error,
    /// Or if the return value cannot be deserialized into the requested type
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
    ///
    /// # Errors
    /// Can fail if the function is not found, if the function returns an error,
    /// Or if the return value cannot be deserialized into the requested type
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
    ///
    /// # Errors
    /// Can fail if the value is not found or if the value cannot be deserialized into the requested type
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
impl AsRef<Worker<DefaultWorker>> for DefaultWorker {
    fn as_ref(&self) -> &Worker<DefaultWorker> {
        &self.0
    }
}

/// Options for the default worker
#[derive(Default, Clone)]
pub struct DefaultWorkerOptions {
    /// The default entrypoint function to use if none is registered
    pub default_entrypoint: Option<String>,

    /// The timeout to use for the runtime
    pub timeout: std::time::Duration,

    /// Optional snapshot to load into the runtime
    /// This will reduce load times, but requires the same extensions to be loaded
    /// as when the snapshot was created
    pub startup_snapshot: Option<&'static [u8]>,

    /// Optional shared array buffer store to use for the runtime
    /// Allows data-sharing between runtimes across threads
    pub shared_array_buffer_store: Option<deno_core::SharedArrayBufferStore>,
}

/// Query types for the default worker
#[derive(Debug, Clone)]
pub enum DefaultWorkerQuery {
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
#[derive(Debug, Clone)]
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
