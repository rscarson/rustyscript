///
/// This example shows how to use `Runtime::eval` to run async code
/// Note that there is no support for top-level await but you can use `Promise` to work around this
///
use rustyscript::{js_value::Promise, Error, Runtime};

fn main() -> Result<(), Error> {
    // Create a new runtime
    let mut runtime = Runtime::new(Default::default())?;
    let tokio_runtime = runtime.tokio_runtime();

    // A little setup for later
    // The `::<()>` is a type hint to the compiler that we don't need a return value
    // Previously it could be left out, but now it will cause a warning, and in the future an error
    runtime.eval::<()>(
        "globalThis.sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));",
    )?;

    // Can be run as blocking
    runtime.eval::<u32>("sleep(1000).then(() => 1)")?;

    // Or as async
    let future = async {
        let result: Promise<u32> = runtime.eval_immediate("sleep(1000).then(() => 2)").await?;
        result.into_future(&mut runtime).await?;

        Ok::<(), Error>(())
    };
    tokio_runtime.block_on(future)?;

    Ok(())
}
