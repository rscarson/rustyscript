///
/// rustyscript is not thread-safe
/// This is due to a limitation of the underlying engine, deno_core
///
/// We can slither around that limitation however, like this!
/// Use this method with extreme caution, and only if you understand
/// how it works
///
use rustyscript::{json_args, module, Error, Runtime, StaticModule};
use std::cell::{OnceCell, RefCell};

// Create a thread-local version of the runtime
// This allows the following to be enforced:
// - thread_local: Runtime is not sent between threads
// - OnceCell: Runtime is only initialized once
// - RefCell: Runtime is never accessed concurrently
thread_local! {
    static RUNTIME_CELL: OnceCell<RefCell<Runtime>> = OnceCell::new();
}

/// Perform an operation on the runtime instance
/// Will return T if we can get access to the runtime
/// or panic went wrong
fn with_runtime<T, F: FnMut(&mut Runtime) -> T>(mut callback: F) -> T {
    RUNTIME_CELL.with(|once_lock| {
        let rt_mut = once_lock.get_or_init(|| {
            RefCell::new(Runtime::new(Default::default()).expect("could not create the runtime"))
        });
        let mut runtime = rt_mut.borrow_mut();
        callback(&mut runtime)
    })
}

// Modules can be defined statically using this macro!
static MY_MODULE: StaticModule = module!(
    "custom_types.js",
    "
    export const my_function = () => 'test';
"
);

fn main() -> Result<(), Error> {
    let value: String = with_runtime(|runtime| {
        let module_context = runtime.load_module(&MY_MODULE.to_module())?;
        runtime.call_function(&module_context, "my_function", json_args!())
    })?;

    assert_eq!(value, "test");
    Ok(())
}
