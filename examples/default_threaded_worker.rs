///
/// This example shows how to use the threaded worker feature using the default worker implementation
/// In this example we load a module, and execute a function from it
///
use rustyscript::{json_args, worker::DefaultWorker, Error, Module};

fn main() -> Result<(), Error> {
    let worker = DefaultWorker::new(Default::default())?;

    let module = Module::new("test.js", "export function add(a, b) { return a + b; }");
    let module_id = worker.load_module(module)?;

    let result: i32 = worker.call_function(
        Some(module_id),
        "add".to_string(),
        json_args!(1, 2).to_vec(),
    )?;
    assert_eq!(result, 3);
    Ok(())
}
