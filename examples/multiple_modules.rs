//!
//! This example demonstrates the use of multiple modules in rustyscript, in a single runtime
//!
use rustyscript::{json_args, module, Error, Module, Runtime, RuntimeOptions, Undefined};

// This time we will embed this module into the executable directly.
// After all, it is a very small file we will always need - why
// take the extra overhead for the filesystem!
const API_MODULE: Module = module!(
    "examples/get_value.ts",
    "
    let my_internal_value:number;
    
    export function getValue():number {
      return my_internal_value;
    }
    
    export function setValue(value:number) {
      my_internal_value = value * 2;
    }
  "
);

fn main() -> Result<(), Error> {
    // First we need a runtime. There are a handful of options available
    // here but the one we need right now is default_entrypoint.
    // This tells the runtime that a function is needed for initial
    // setup of our runtime.
    let mut runtime = Runtime::new(RuntimeOptions {
        default_entrypoint: Some("setValue".to_string()),
        ..Default::default()
    })?;

    // Now we can include our static module - `to_module()` is needed
    // to pull into into a form we can use.
    //
    // The `call_entrypoint` function will call our module's setValue
    // function for us - the function was found and a reference to it
    // stored in advance on load so that this function call can be
    // made with less overhead.
    // Just like before, `::<Undefined` means we do not care if the
    // function returns a result.
    let module_handle = runtime.load_module(&API_MODULE)?;
    runtime.call_entrypoint::<Undefined>(&module_handle, json_args!(2))?;

    // Now we can load our new module from the filesystem
    // The handle that load_module returns is used to give context to future calls
    let use_value_handle =
        runtime.load_module(&Module::load("examples/javascript/multiple_modules.js")?)?;

    // We use the returned handle to extract the const that it exports!
    // We tell the compiler we'd like it as a string, and give the name of the value
    // We'd like to retrieve!
    let final_value: String = runtime.get_value(Some(&use_value_handle), "final_value")?;
    println!("The received value was {final_value}");

    Ok(())
}
