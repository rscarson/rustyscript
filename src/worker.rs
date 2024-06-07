//! Provides a worker thread that can be used to run javascript code in a separate thread through a channel pair

use crate::Error;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};

/// A worker thread that can be used to run javascript code in a separate thread
/// Contains a channel pair for communication, and a single runtime instance
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
///
/// Example:
/// ```rust
/// use rustyscript::{Worker, InnerWorker, RuntimeOptions, Runtime, Error};
///
/// enum Query {
///     Call(String, Vec<serde_json::Value>),
///     Stop,
/// }
///
/// enum Response {
///    Value(serde_json::Value),
///    Error(Error),
/// }
///
/// struct MyWorker;
/// impl InnerWorker for MyWorker {
///     type Runtime = Runtime;
///     type RuntimeOptions = RuntimeOptions;
///     type Query = Query;
///     type Response = Response;
///
///     fn init_runtime(options: Self::RuntimeOptions) -> Result<Self::Runtime, Error> {
///         Runtime::new(options)
///     }
///
///     fn thread(
///         runtime: Self::Runtime,
///         rx: Receiver<Self::Query>,
///         tx: Sender<Self::Response>,
///     ) {
///            loop {
///                let msg = match rx.recv() {
///                    Ok(msg) => msg,
///                    Err(_) => break,
///                };
///                
///                match msg {
///                    Query::Call(name, args) => {
///                        let result = runtime.call(name, args);
///                        tx.send(match result {
///                            Ok(v) => Response::Value(v),
///                            Err(e) => Response::Error(e),
///                        }).unwrap();
///                    }
///
///                    Query::Stop => break,
///                }
///            }
///        }
/// }
///
/// fn main() -> Result<(), Error> {
///     let worker = Worker::<MyWorker>::new()?;
///     let value = match worker.send_and_await(Query::Call("my_function".to_string(), vec![]))? {
///         Ok(Response::Value(v)) => Ok(v),
///         Ok(Response::Error(e)) => Err(e),
///     }?;
///
///     println!("Result: {:?}", value);
///     worker.send(Query::Stop)?;
///     worker.join()?;
///     Ok(())
/// }
/// ```
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

    /// The main thread function that will be run by the worker
    /// This should handle all incoming queries and send responses back
    /// Normally takes this form:
    /// ```no_run
    /// loop {
    ///     let msg = match rx.recv() {
    ///         Ok(msg) => msg,
    ///         Err(_) => break,
    ///     };
    ///
    ///     match msg {
    ///         [...]
    ///     }
    /// }
    /// ```
    fn thread(runtime: Self::Runtime, rx: Receiver<Self::Query>, tx: Sender<Self::Response>);
}

/// A worker implementation that uses the default runtime
/// This is the simplest way to use the worker, as it requires no additional setup
/// It attempts to provide as much functionality as possible from the standard runtime
///
/// Please note that it uses serde_json::Value for queries and responses, which comes with a performance cost
/// For a more performant worker, or to use extensions and/or loader caches, you'll need to implement your own worker
pub struct DefaultWorker(Worker<DefaultWorker>);
impl InnerWorker for DefaultWorker {
    type Runtime = crate::Runtime;
    type RuntimeOptions = DefaultWorkerOptions;
    type Query = DefaultWorkerQuery;
    type Response = DefaultWorkerResponse;

    fn init_runtime(options: Self::RuntimeOptions) -> Result<Self::Runtime, Error> {
        crate::Runtime::new(crate::RuntimeOptions {
            default_entrypoint: options.default_entrypoint,
            timeout: options.timeout,
            ..Default::default()
        })
    }

    fn thread(mut runtime: Self::Runtime, rx: Receiver<Self::Query>, tx: Sender<Self::Response>) {
        loop {
            let msg = match rx.recv() {
                Ok(msg) => msg,
                Err(_) => break,
            };

            match msg {
                DefaultWorkerQuery::Stop => {
                    tx.send(Self::Response::Ok(())).unwrap();
                    break;
                }

                DefaultWorkerQuery::RegisterFunction(name, func) => {
                    match runtime.register_function(&name, func) {
                        Ok(_) => tx.send(Self::Response::Ok(())).unwrap(),
                        Err(e) => tx.send(Self::Response::Error(e)).unwrap(),
                    }
                }

                DefaultWorkerQuery::Eval(code) => match runtime.eval(&code) {
                    Ok(v) => tx.send(Self::Response::Value(v)).unwrap(),
                    Err(e) => tx.send(Self::Response::Error(e)).unwrap(),
                },
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
    pub fn eval(&self, code: String) -> Result<crate::serde_json::Value, Error> {
        match self.0.send_and_await(DefaultWorkerQuery::Eval(code))? {
            DefaultWorkerResponse::Value(v) => Ok(v),
            DefaultWorkerResponse::Error(e) => Err(e),
            _ => Err(Error::Runtime(
                "Unexpected response from the worker".to_string(),
            )),
        }
    }
}

/// Options for the default worker
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
}

/// Response types for the default worker
pub enum DefaultWorkerResponse {
    /// A successful response with a value
    Value(crate::serde_json::Value),

    /// A successful response with no value
    Ok(()),

    /// An error response
    Error(Error),
}
