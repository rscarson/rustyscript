///
/// This example shows the creation of a custom runtime worker
/// This worker will be able to execute blocks of non-emca javascript
///
use rustyscript::serde_json::Value;
use rustyscript::{
    worker::{InnerWorker, Worker},
    Error, Runtime, RuntimeOptions,
};
use std::sync::mpsc::{Receiver, Sender};

/// The set of queries that can be sent to the worker
enum Query {
    Eval(String),
    Stop,
}

/// The set of responses that can be received from the worker
enum Response {
    Value(Value),
    Error(Error),
}

/// The options used to initialize the runtime
struct Options {
    timeout: std::time::Duration,
}

struct MyWorker;
impl InnerWorker for MyWorker {
    // The types used by the worker
    type Runtime = Runtime;
    type RuntimeOptions = Options;
    type Query = Query;
    type Response = Response;

    fn init_runtime(options: Self::RuntimeOptions) -> Result<Self::Runtime, Error> {
        Runtime::new(RuntimeOptions {
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
                // Evaluate the code and send the result back
                Query::Eval(code) => {
                    let result = runtime.eval(&code);
                    tx.send(match result {
                        Ok(v) => Response::Value(v),
                        Err(e) => Response::Error(e),
                    })
                    .unwrap();
                }

                // Stop the worker
                Query::Stop => break,
            }
        }
    }
}

fn main() -> Result<(), Error> {
    // Create a new worker
    let worker = Worker::<MyWorker>::new(Options {
        timeout: std::time::Duration::from_millis(1000),
    })?;

    // Evaluate a simple expression
    let value = match worker.send_and_await(Query::Eval("1 + 1".to_string()))? {
        Response::Value(v) => Ok(v),
        Response::Error(e) => Err(e),
    }?;
    println!("Result: {:?}", value);

    // Stop the worker
    worker.send(Query::Stop)?;
    worker.join()?;

    Ok(())
}
