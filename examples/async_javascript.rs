///
/// This example shows the use of async functions in JS
/// Here we have a module with 4 async functions that resolve after a given time
/// We call them in sequence and await the results
///
/// Notes:
/// - When using the async variants of the functions it is important to complete the JS event loop with [Runtime::await_event_loop]
/// - Async variants will wait for the function's return value to resolve, but will not wait for the event loop to complete
///
use rustyscript::{json_args, Error, Module, Runtime, Undefined};

fn main() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        "
        function resolve_after(t) {
            new Promise((resolve) => {
                setTimeout(() => {
                    console.log('Finished after ' + t + 'ms');
                    resolve('resolved');
                }, t);
            });
        }
        export const f1 = async () => resolve_after(8000);
        export const f2 = async () => resolve_after(4000);
        export const f3 = async () => resolve_after(2000);
        export const f4 = async () => resolve_after(1000);
    ",
    );

    let mut runtime = Runtime::new(Default::default())?;
    runtime.load_module(&module)?;

    let handle = runtime.load_module(&module)?;
    let tokio_runtime = runtime.tokio_runtime();

    //
    // Now we await the results
    //

    let future = async move {
        runtime
            .call_async_function::<Undefined>(Some(&handle), "f1", json_args!())
            .await?;
        runtime
            .call_async_function::<Undefined>(Some(&handle), "f2", json_args!())
            .await?;
        runtime
            .call_async_function::<Undefined>(Some(&handle), "f3", json_args!())
            .await?;
        runtime
            .call_async_function::<Undefined>(Some(&handle), "f4", json_args!())
            .await?;
        runtime.await_event_loop(Default::default()).await
    };
    tokio_runtime.block_on(async move { future.await })?;

    Ok(())
}
