use rustyscript::{json_args, Error, Module, Runtime, RuntimeOptions};
///
/// This example shows features requiring the 'web' feature to work
/// Stuff like setTimeout, atob/btoa, file reads and fetch are all examples
///
/// We will focus on timers and fetch here
///
use std::time::Duration;

fn main() -> Result<(), Error> {
    // This module has an async function, which is not itself a problem
    // However, it uses setTimeout - the timer will never be triggered
    // unless the web feature is active.
    // See above for a longer list for web feature exclusives
    let module = Module::new(
        "test.js",
        "
        const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
        export async function test() {
            await sleep(10);
            return 2;
        }

        export async function fetch_example() {
            return new Promise((accept, reject) => {
                fetch('https://api.github.com/users/mralexgray/repos', {
                    method: 'GET',
                    headers: {
                      Accept: 'application/json',
                    },
                  }).then(response => response.json())
                  .then(json => accept(json))
                  .catch(e => reject(e));
            });
        }
        ",
    );

    // We add a timeout to the runtime anytime async might be used
    let mut runtime = Runtime::new(RuntimeOptions {
        timeout: Duration::from_millis(1000),
        ..Default::default()
    })?;

    // The async function
    let module_handle = runtime.load_module(&module)?;
    let value: usize = runtime.call_function(&module_handle, "test", json_args!())?;
    assert_eq!(value, 2);

    // Fetch example
    let data: rustyscript::serde_json::Value =
        runtime.call_function(&module_handle, "fetch_example", json_args!())?;
    println!("{:?}", data);
    Ok(())
}
