use deno_core::serde_json;
///
/// This example shows how to use the threaded worker feature using a custom implementation of a worker
/// We will create a basic worker implementation able to execute snippets of non-emca JS
///
use rustyscript::{
    worker::{InnerWorker, Worker},
    Error, Runtime,
};

fn main() -> Result<(), Error> {
    let worker = MyWorker::new(MyWorkerOptions {
        timeout: std::time::Duration::from_secs(1),
    })?;

    let result: i32 = worker.execute("1 + 2")?;
    assert_eq!(result, 3);

    Ok(())
}

/// The worker implementation
/// We will have instances supertype the Worker itself
/// so can just instantiate this struct directly
pub struct MyWorker(Worker<MyWorker>);

impl MyWorker {
    /// Create a new instance of the worker
    pub fn new(options: MyWorkerOptions) -> Result<Self, Error> {
        Ok(Self(Worker::new(options)?))
    }

    /// Execute a snippet of JS code on our threaded worker
    pub fn execute<T>(&self, code: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        match self
            .0
            .send_and_await(MyWorkerMessage::Execute(code.to_string()))?
        {
            MyWorkerMessage::Value(v) => Ok(serde_json::from_value(v)?),
            MyWorkerMessage::Error(e) => Err(e),
            _ => Err(Error::Runtime("Unexpected response".to_string())),
        }
    }
}

/// The messages we will use to communicate with the worker
pub enum MyWorkerMessage {
    Execute(String),

    Error(Error),
    Value(serde_json::Value),
}

/// The runtime options for our worker
pub struct MyWorkerOptions {
    pub timeout: std::time::Duration,
}

// Our implementation of the InnerWorker trait
// This is where we define how the worker will handle queries
// Here we are using the same message type for queries and responses
// and using the default runtime
impl InnerWorker for MyWorker {
    type Query = MyWorkerMessage;
    type Response = MyWorkerMessage;
    type RuntimeOptions = MyWorkerOptions;
    type Runtime = Runtime;

    /// Initialize the runtime using the options provided
    fn init_runtime(options: Self::RuntimeOptions) -> Result<Self::Runtime, Error> {
        Runtime::new(rustyscript::RuntimeOptions {
            timeout: options.timeout,
            ..Default::default()
        })
    }

    /// Handle all possible queries
    fn handle_query(runtime: &mut Self::Runtime, query: Self::Query) -> Self::Response {
        match query {
            MyWorkerMessage::Execute(code) => match runtime.eval::<serde_json::Value>(&code) {
                Ok(value) => MyWorkerMessage::Value(value),
                Err(e) => MyWorkerMessage::Error(e),
            },

            MyWorkerMessage::Error(e) => MyWorkerMessage::Error(e),
            MyWorkerMessage::Value(v) => MyWorkerMessage::Value(v),
        }
    }
}
