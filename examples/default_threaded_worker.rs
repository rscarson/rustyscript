///
/// This example shows how to use the threaded worker feature using the default worker implementation
/// In this example we load a module, and execute a function from it
///
use rustyscript::{worker::DefaultWorker, Error, Module};

fn main() -> Result<(), Error> {
    let worker = DefaultWorker::new(Default::default())?;

    let module = Module::new("test.js", "export function add(a, b) { return a + b; }");
    let module_id = worker.load_module(module)?;

    let result: i32 =
        worker.call_function(Some(module_id), "add".to_string(), vec![1.into(), 2.into()])?;
    assert_eq!(result, 3);
    Ok(())
}
