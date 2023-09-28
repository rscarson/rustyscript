///
/// This example shows how to add deno_core extensions into the runtime.
/// 
/// Extensions like the one below allow you to let JS code call functions
/// in rust
/// 
/// Extensions consist of a set of #[op2] functions, an extension! macro,
/// and one or more optional JS modules.
/// 
use js_playground::deno_core::{ op2, extension };
use js_playground::{Runtime, RuntimeOptions, Script, Error};

#[op2]
#[bigint]
fn op_add_example(#[bigint] a: i64, #[bigint] b: i64) -> i64 {
    a + b
}

extension!(
    example_ext,
    ops = [op_add_example],
    esm_entry_point = "ext:example_ext/runtime_extensions.js",
    esm = [ dir "examples", "runtime_extensions.js" ],
);

fn main() -> Result<(), Error> {
    let script = Script::new(
        "test.js",
        " export const result = example_ext.add(5, 5); "
    );

    let mut runtime = Runtime::new(RuntimeOptions {
        extensions: vec![example_ext::init_ops_and_esm()],
        ..Default::default()
    })?;
    let module_handle = runtime.load_modules(script, vec![])?;

    let result: i64 = runtime.get_value(&module_handle, "result")?;
    assert_eq!(10, result);
    Ok(())
}