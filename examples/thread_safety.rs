///
/// rustyscript is not thread-safe
/// This is due to a limitation of the underlying engine, deno_core
/// However, rustyscript provides a mechanism to safely use it in a static context
///
/// See `examples/default_threaded_worker` and `examples/custom_threaded_worker`
/// for a more flexible way to run rustyscript in a threaded environment
///
use rustyscript::{module, static_runtime, Error, RuntimeOptions, StaticModule};
use std::time::Duration;

static_runtime!(RUNTIME, {
    RuntimeOptions {
        timeout: Duration::from_secs(5),
        ..Default::default()
    }
});

// Modules can be defined statically using this macro!
static MY_MODULE: StaticModule = module!(
    "custom_types.js",
    "
    export const my_function = () => 'test';
"
);

fn main() -> Result<(), Error> {
    let value: String = RUNTIME::with(|runtime| {
        let module_context = runtime.load_module(&MY_MODULE.to_module())?;
        runtime.call_function(Some(&module_context), "my_function", &())
    })?;

    assert_eq!(value, "test");
    Ok(())
}
