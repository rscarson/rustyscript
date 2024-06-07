use rustyscript::serde_json::Value;
use std::sync::mpsc::{Receiver, Sender};

///
/// This example shows the creation of a custom runtime worker
/// This worker will be able to execute blocks of non-emca javascript
///
use rustyscript::{
    worker::{InnerWorker, Worker},
    Error, Runtime, RuntimeOptions,
};

enum Query {
    Eval(String),
    Stop,
}

enum Response {
    Value(Value),
    Error(Error),
}

struct Options {
    timeout: std::time::Duration,
}

struct MyWorker;
impl InnerWorker for MyWorker {
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
                Query::Eval(code) => {
                    let result = runtime.eval(&code);
                    tx.send(match result {
                        Ok(v) => Response::Value(v),
                        Err(e) => Response::Error(e),
                    })
                    .unwrap();
                }

                Query::Stop => break,
            }
        }
    }
}

fn main() -> Result<(), Error> {
    let worker = Worker::<MyWorker>::new(Options {
        timeout: std::time::Duration::from_millis(1000),
    })?;

    let value = match worker.send_and_await(Query::Eval("1 + 1".to_string()))? {
        Response::Value(v) => Ok(v),
        Response::Error(e) => Err(e),
    }?;

    println!("Result: {:?}", value);
    worker.send(Query::Stop)?;
    worker.join()?;
    Ok(())
}
