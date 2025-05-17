use super::ExtensionTrait;
use crate::{error::Error, RsAsyncFunction, RsFunction};
use deno_core::{extension, op2, serde_json, v8, Extension, OpState};
use std::collections::HashMap;

type FnCache = HashMap<String, Box<dyn RsFunction>>;
type AsyncFnCache = HashMap<String, Box<dyn RsAsyncFunction>>;

mod callbacks;

/// Registers a JS function with the runtime as being the entrypoint for the module
///
/// # Arguments
/// * `state` - The runtime's state, into which the function will be put
/// * `callback` - The function to register
#[op2]
fn op_register_entrypoint(state: &mut OpState, #[global] callback: v8::Global<v8::Function>) {
    state.put(callback);
}

#[op2]
#[serde]
#[allow(clippy::needless_pass_by_value)]
fn call_registered_function(
    #[string] name: &str,
    #[serde] args: Vec<serde_json::Value>,
    state: &mut OpState,
) -> Result<serde_json::Value, Error> {
    if state.has::<FnCache>() {
        let table = state.borrow_mut::<FnCache>();
        if let Some(callback) = table.get(name) {
            return callback(&args);
        }
    }

    Err(Error::ValueNotCallable(name.to_string()))
}

#[op2(async)]
#[serde]
fn call_registered_function_async(
    #[string] name: String,
    #[serde] args: Vec<serde_json::Value>,
    state: &mut OpState,
) -> impl std::future::Future<Output = Result<serde_json::Value, Error>> {
    if state.has::<AsyncFnCache>() {
        let table = state.borrow_mut::<AsyncFnCache>();
        if let Some(callback) = table.get(&name) {
            return callback(args);
        }
    }

    Box::pin(std::future::ready(Err(Error::ValueNotCallable(name))))
}

#[op2(fast)]
fn op_panic2(#[string] msg: &str) -> Result<(), Error> {
    Err(Error::Runtime(msg.to_string()))
}

extension!(
    rustyscript,
    ops = [op_register_entrypoint, call_registered_function, call_registered_function_async],
    esm_entry_point = "ext:rustyscript/rustyscript.js",
    esm = [ dir "src/ext/rustyscript", "rustyscript.js" ],
    middleware = |op| match op.name {
        "op_panic" => op.with_implementation_from(&op_panic2()),
        _ => op,
    }
);
impl ExtensionTrait<()> for rustyscript {
    fn init(options: ()) -> Extension {
        rustyscript::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![rustyscript::build((), is_snapshot)]
}
