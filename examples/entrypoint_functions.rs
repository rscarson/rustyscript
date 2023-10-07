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
use js_playground::{json_args, Error, Runtime, RuntimeOptions, Script, Undefined};

fn main() -> Result<(), Error> {
    let script = Script::new(
        "test.js",
        "
        let internalValue = 0;
        export const getValue = () => internalValue;
        export function setUp(value) {
            internalValue = value * 2;
        }
        ",
    );

    let mut runtime = Runtime::new(RuntimeOptions {
        default_entrypoint: Some("setUp".to_string()),
        ..Default::default()
    })?;
    let module_handle = runtime.load_module(&script)?;

    runtime.call_entrypoint::<Undefined>(&module_handle, &[Runtime::arg(2)])?;

    let internal_value: usize = runtime.call_function(&module_handle, "getValue", json_args!())?;
    assert_eq!(4, internal_value);
    Ok(())
}
