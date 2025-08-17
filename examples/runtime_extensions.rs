//!
//! This example shows how to add deno_core extensions into the runtime.
//!
//! Extensions like the one being used (see examples/ext/example_extension.rs)
//! allow you to call rust code from within JS
//!
//! Extensions consist of a set of #[op2] functions, an extension! macro,
//! and one or more optional JS modules.
//!
use rustyscript::{Error, Module, Runtime, RuntimeOptions};
use std::collections::HashSet;

mod example_extension;

fn main() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        r#"
        import { add } from "example:calculator";
        export const result = add(5, 5);
    "#,
    );

    // Whitelist the example: schema for the module
    let mut schema_whlist = HashSet::new();
    schema_whlist.insert("example:".to_string());

    // We provide a function returning the set of extensions to load
    // It needs to be a function, since deno_core does not currently
    // allow clone or copy from extensions
    let mut runtime = Runtime::new(RuntimeOptions {
        schema_whlist,
        extensions: vec![example_extension::example_extension::init()],
        ..Default::default()
    })?;
    let module_handle = runtime.load_module(&module)?;

    let result: i64 = runtime.get_value(Some(&module_handle), "result")?;
    assert_eq!(10, result);

    Ok(())
}
