///
/// This example is meant to demonstrate the use of the runtime state, as well as the
/// registration of rust functions that are callable from JS
///
use rustyscript::{async_callback, serde_json, sync_callback, Error, Module, Runtime};

fn main() -> Result<(), Error> {
    // Let's get a new runtime first
    let mut runtime = Runtime::new(Default::default())?;

    // We can use a normal function, if we wish
    // It can also be `move` if we want to capture some state
    runtime.register_function("echo", |args| {
        // Decode the input
        let input = args
            .first()
            .ok_or(Error::Runtime("No input".to_string()))
            .map(|v| serde_json::from_value::<String>(v.clone()))??;

        // Encode the output
        let output = format!("Hello, {input}!");
        Ok::<_, Error>(serde_json::Value::String(output))
    })?;

    // There is also a helper macro to create a callback
    // It will take care of deserializing arguments and serializing the result
    runtime.register_function(
        "add",
        sync_callback!(|a: i64, b: i64| {
            a.checked_add(b)
                .ok_or(Error::Runtime("Overflow".to_string()))
        }),
    )?;

    // There is also an async version
    runtime.register_async_function(
        "asyncEcho",
        async_callback!(|input: String| {
            async move { Ok::<_, Error>(format!("Hello, {input}!")) }
        }),
    )?;

    // A module that will consume the functions we registered

    // Our module will simply call a rust-side function
    let module = Module::new(
        "test.js",
        "

        let echo = rustyscript.functions['echo'];
        let add = rustyscript.functions['add'];
        let asyncEcho = rustyscript.async_functions['asyncEcho'];

        console.log(echo('world'));
        console.log(add(5, 6));
        asyncEcho('foo').then(console.log);
    ",
    );

    // Now we call the function from JS and make sure everything worked
    // I'll make any errors prettier here
    match runtime.load_module(&module) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}", e.as_highlighted(Default::default()));
        }
    }

    Ok(())
}
