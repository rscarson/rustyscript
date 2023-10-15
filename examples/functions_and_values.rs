///
/// This example demonstrates how to extract values, and call functions
/// from rust into JS
///
/// The sample below extracts a value which is deserialized to a custom struct
/// as well as calling a function in JS from rust
///
use rustyscript::{deno_core::serde::Deserialize, json_args, Error, Module, Runtime};

#[derive(PartialEq, Debug, Deserialize)]
struct MyStruct {
    name: String,
    value: usize,
}

fn main() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        "
        export function test(value) {
            return `foo: ${value}`;
        }

        export const bar = {
            'name': 'test',
            'value': 7
        };
        ",
    );

    // Creation of a new runtime, using the default options
    let mut runtime = Runtime::new(Default::default())?;

    // Import the module
    // This returns a handle which is used to contextualize future calls
    // This ensures you get access to the exports for the module
    let module_handle = runtime.load_module(&module)?;

    // Calling an exported function
    // This will also work with anything in the global scope (eg: globalThis)
    let function_value: String = runtime.call_function(&module_handle, "test", json_args!("A"))?;
    assert_eq!(function_value, "foo: A");

    // Custom types can be exported from JS easily!
    let value: MyStruct = runtime.get_value(&module_handle, "bar")?;
    assert_eq!(
        MyStruct {
            name: "test".to_string(),
            value: 7
        },
        value
    );

    Ok(())
}
