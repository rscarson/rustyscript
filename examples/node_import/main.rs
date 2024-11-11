///
/// This example demonstrates importing and using node modules
///
/// 2 node modules are imported in this example:
/// - `os` from the Deno polyfills to the node standard library
/// - `chalk` from npm, it will look for a matching package in the node_modules directory
///
use rustyscript::{Error, Module, Runtime, RuntimeOptions};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
    }
}

fn run() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        r#"
            // From the node standard library (Deno polyfills)
            import os from "node:os";

            // From npm
            import chalk from "npm:chalk@5";

            export function print_hostname() {
                console.log("Getting hostname from node:os:");
                console.log(chalk.blue(os.hostname()));
            }
        "#,
    );

    // First we need a runtime, and to load the module we just created
    // We set the current directory to the examples/node_import directory
    // so that `node_modules` can be found
    let mut runtime = Runtime::new(RuntimeOptions::default())?;
    runtime.set_current_dir("examples/node_import")?;
    let module_handle = runtime.load_module(&module)?;

    // Now we can call the function we defined in the module
    // `::<()>` specifies that we don't expect any return value
    // This previously was deduced as `!` by the compiler, but that
    // feature is now being deprecated
    runtime.call_function::<()>(Some(&module_handle), "print_hostname", &())?;

    Ok(())
}
