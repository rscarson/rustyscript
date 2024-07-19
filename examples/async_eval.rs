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
    runtime
        .eval("globalThis.sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));")?;

    let future = async {
        let result: Promise<u32> = runtime.eval("sleep(1000).then(() => 2)")?;
        result.into_future(&mut runtime).await?;

        Ok::<(), Error>(())
    };
    tokio_runtime.block_on(future)?;

    Ok(())
}
