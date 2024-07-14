///
/// This example shows the use of async functions in JS
///
/// Here we have a module with 4 async functions that resolve after a given time
/// We call them in sequence and await the results
///
/// Notes:
/// - When using the async variants of the functions it is important to complete the JS event loop with [Runtime::await_event_loop]
/// - Async variants will wait for the function's return value to resolve, but will not wait for the event loop to complete
///
use rustyscript::{js_value::Promise, json_args, Error, Module, Runtime};

fn main() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        "
        function resolve_after(t) {
            return new Promise((resolve) => {
                return setTimeout(() => {
                    console.log('Finished after ' + t + 'ms');
                    resolve('resolved');
                }, t);
            });
        }
        export const f1 = async () => resolve_after(4000);
        export const f2 = async () => resolve_after(2000);
        export const f3 = async () => resolve_after(1000);
    ",
    );

    // Create a new runtime
    let mut runtime = Runtime::new(Default::default())?;
    let handle = runtime.load_module(&module)?;
    let tokio_runtime = runtime.tokio_runtime();

    //
    // In this version we await the promises in sequence
    // They will be resolved in the order they were called
    //
    let future = async {
        let v1: String = runtime
            .call_function_async(Some(&handle), "f1", json_args!())
            .await?;
        let v2: String = runtime
            .call_function_async(Some(&handle), "f2", json_args!())
            .await?;
        let v3: String = runtime
            .call_function_async(Some(&handle), "f3", json_args!())
            .await?;

        println!("v1={}\nv2={}\nv3={}", v1, v2, v3);

        Ok::<(), Error>(())
    };
    tokio_runtime.block_on(async move { future.await })?;

    // Another way to do it is to export the promises first
    // Then await them in sequence after running the event loop
    // This is useful when you need to store the promises for later
    //
    // Normally calling a function would resolve the promise,
    // Causing the future to borrow the runtime mutably
    // So in order to store all the promises at once, we need to call `call_function_immediate`
    // Which will not resolve the event loop
    let p1: Promise<String> = runtime.call_function_immediate(Some(&handle), "f1", json_args!())?;
    let p2: Promise<String> = runtime.call_function_immediate(Some(&handle), "f2", json_args!())?;
    let p3: Promise<String> = runtime.call_function_immediate(Some(&handle), "f3", json_args!())?;

    // Now we can convert the promises back into futures
    // And await them in sequence
    // Converting them into the actual values we want
    let future = async {
        println!(
            "p1={}\np2={}\np3={}",
            p1.into_future(&mut runtime).await?,
            p2.into_future(&mut runtime).await?,
            p3.into_future(&mut runtime).await?,
        );

        Ok::<(), Error>(())
    };
    tokio_runtime.block_on(async move { future.await })?;

    Ok(())
}
