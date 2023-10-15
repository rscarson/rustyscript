///
/// This example shows a basic usage of the runtime
///
/// The call to Runtime::execute_module is a one liner which:
/// - Creates a new runtime with the given options
/// - Loads the modules passed in
/// - Calls the module's entrypoint function
///     - Either a default passed as an option
///     - Or a call in JS to rustyscript.register_entrypoint
/// - Returns the result
///
/// Instead of just vec![], one could pass in other JS modules
/// which could be imported using `import './filename.js';`
///
use rustyscript::{json_args, Error, Module, Runtime};

fn main() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        "
        rustyscript.register_entrypoint(
            (string, integer) => {
                console.log(`Hello world: string=${string}, integer=${integer}`);
                return 2;
            }
        )
        ",
    );

    // Execute the module above as an ES module
    // Do not side-load any additional modules
    // Use the default Runtime options
    // Pass 2 args into the entrypoint function
    // And expect a usize back from it
    let value: usize =
        Runtime::execute_module(&module, vec![], Default::default(), json_args!("test", 5))?;

    assert_eq!(value, 2);
    Ok(())
}
