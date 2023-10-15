///
/// This example demonstrates how to extract values, and call functions
/// from rust into JS
///
/// The sample below extracts a value which is deserialized to a custom struct
/// as well as calling a function in JS from rust
///
use rustyscript::{deno_core::serde::Deserialize, Error, Module, Runtime};

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

    let mut runtime = Runtime::new(Default::default())?;
    let module_handle = runtime.load_module(&module)?;

    let function_value: String =
        runtime.call_function(&module_handle, "test", &[Runtime::arg("A")])?;
    assert_eq!(function_value, "foo: A");

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
