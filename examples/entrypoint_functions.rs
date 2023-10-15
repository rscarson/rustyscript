///
/// This example is meant to demonstrate the basic usage of entrypoint functions
///
/// A module can optionally have an entrypoint function (that can return a value and accept args)
/// which can be called from rust on load.
///
/// The same effect can be achieved by calling a function later, so they are optional
/// They are most useful in the context of Runtime::execute_module, which can be seen
/// in the 'hello_world' example.
///
use rustyscript::{json_args, Error, Module, Runtime, RuntimeOptions, Undefined};

fn main() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        "
        let internalValue = 0;
        export const getValue = () => internalValue;
        export function setUp(value) {
            internalValue = value * 2;
        }
        ",
    );

    // Let's get a new runtime that defaults to the setUp function as the entrypoint
    // and load our ES module into it
    let mut runtime = Runtime::new(RuntimeOptions {
        default_entrypoint: Some("setUp".to_string()),
        ..Default::default()
    })?;
    let module_handle = runtime.load_module(&module)?;

    // We call the entrypoint - Undefined just means we don't care about
    // the return type here
    runtime.call_entrypoint::<Undefined>(&module_handle, json_args!(2))?;

    // Now the setUp is done, and the internal value is ready for use
    let internal_value: usize = runtime.call_function(&module_handle, "getValue", json_args!())?;
    assert_eq!(4, internal_value);
    Ok(())
}
