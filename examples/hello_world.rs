///
/// This example shows a basic usage of the runtime
///
/// The call to Runtime::execute_module is a one liner which:
/// - Creates a new runtime with the given options
/// - Loads the scripts passed in
/// - Calls the script's entrypoint function
///     - Either a default passed as an option
///     - Or a call in JS to js_playground.register_entrypoint
/// - Returns the result
///
/// Instead of just vec![], one could pass in other JS modules
/// which could be imported using `import './filename.js';`
///
use js_playground::{Error, Runtime, Script};

fn main() -> Result<(), Error> {
    let script = Script::new(
        "test.js",
        "
        js_playground.register_entrypoint(
            (string, integer) => {
                console.log(`Hello world: string=${string}, integer=${integer}`);
                return 2;
            }
        )
        ",
    );

    // Execute the script above as an ES module
    // Do not side-load any additional modules
    // Use the default Runtime options
    // Pass no args into the entrypoint function
    // And expect a usize back from it
    let value: usize = Runtime::execute_module(
        &script,
        vec![],
        Default::default(),
        &[Runtime::arg("test"), Runtime::arg(5)],
    )?;

    assert_eq!(value, 2);
    Ok(())
}
