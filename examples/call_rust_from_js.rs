///
/// This example is meant to demonstrate the use of the runtime state, as well as the
/// registration of rust functions that are callable from JS
///
use rustyscript::{serde_json, Error, Module, Runtime};

fn main() -> Result<(), Error> {
    // Our module will simply call a rust-side function
    let module = Module::new("test.js", " rustyscript.functions.setValue('foo'); ");

    // Let's get a new runtime and register a rust callback
    let mut runtime = Runtime::new(Default::default())?;
    runtime.register_function("setValue", |args, state| {
        if let Some(value) = args.get(0) {
            let value: String = serde_json::from_value(value.clone())?;
            state.put(value);
        }
        Ok(serde_json::Value::Null)
    })?;

    // Now we call the function from JS and make sure everything worked
    runtime.load_module(&module)?;
    let my_value: String = runtime.take().unwrap();
    assert_eq!(my_value, "foo");

    Ok(())
}
