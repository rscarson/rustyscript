use deno_core::error::AnyError;
use rustyscript::{include_module, Module, Runtime, RuntimeOptions, StaticModule};
static WHITELIST: StaticModule = include_module!("op_whitelist.js");

fn main() {
    if let Err(e) = check_op_whitelist() {
        eprintln!("Error checking whitelist: {:#?}", e);
        std::process::exit(1);
    } else {
        println!("Whitelist check passed.");
        std::process::exit(0);
    }
}

fn check_op_whitelist() -> Result<(), AnyError> {
    let mut runtime = Runtime::new(RuntimeOptions::default())?;
    runtime.load_module(&WHITELIST.to_module())?;
    let hnd = runtime.load_module(&Module::new(
        "test_whitelist.js",
        "
        import { whitelist } from './op_whitelist.js';
        let ops = Deno.core.ops.op_op_names();
        export const unsafe_ops = ops.filter(op => !whitelist.hasOwnProperty(op));
    ",
    ))?;

    let unsafe_ops: Vec<String> = runtime.get_value(Some(&hnd), "unsafe_ops")?;

    if !unsafe_ops.is_empty() {
        println!("Found unsafe ops: {unsafe_ops:?}.\nOnce confirmed safe, add them to `src/ext/op_whitelist.js`");
        std::process::exit(1);
    }

    Ok(())
}
