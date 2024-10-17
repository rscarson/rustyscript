///
/// This example shows how to set the maximum heap size for the V8 isolate.
/// This is useful when you want to limit the amount of memory a script can consume.
/// A `HeapExhausted` error will be returned if the script exceeds the limit.
///
use rustyscript::{Error, Module, Runtime, RuntimeOptions};

fn main() -> Result<(), Error> {
    // Will exceed the defined heap size
    let module = Module::new(
        "test.js",
        "const largeArray = new Array(40 * 1024 * 1024).fill('a');",
    );

    let mut runtime = Runtime::new(RuntimeOptions {
        max_heap_size: Some(5 * 1024 * 1024),
        ..Default::default()
    })?;

    // Will return a `HeapExhausted` error
    let module_handle = runtime.load_module(&module);

    assert!(module_handle.is_err());

    Ok(())
}
