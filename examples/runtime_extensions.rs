///
/// This example shows how to add deno_core extensions into the runtime.
///
/// Extensions like the one being used (see examples/ext/example_extension.rs)
/// allow you to call rust code from within JS
///
/// Extensions consist of a set of #[op2] functions, an extension! macro,
/// and one or more optional JS modules.
///
use rustyscript::{Error, Module, Runtime, RuntimeOptions};

mod ext;
use ext::example_extension;

fn main() -> Result<(), Error> {
    let module = Module::new("test.js", " export const result = example_ext.add(5, 5); ");

    // We provide a function returning the set of extensions to load
    // It needs to be a function, since deno_core does not currently
    // allow clone or copy from extensions
    let mut runtime = Runtime::new(RuntimeOptions {
        extensions: vec![example_extension::example_extension::init_ops_and_esm()],
        ..Default::default()
    })?;
    let module_handle = runtime.load_module(&module)?;

    let result: i64 = runtime.get_value(Some(&module_handle), "result")?;
    assert_eq!(10, result);
    Ok(())
}
