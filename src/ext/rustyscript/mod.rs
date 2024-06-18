use std::collections::HashMap;

use crate::{error::Error, RsAsyncFunction, RsFunction};
use deno_core::{extension, op2, serde_json, v8, Extension, OpState};

#[op2]
/// Registers a JS function with the runtime as being the entrypoint for the module
///
/// # Arguments
/// * `state` - The runtime's state, into which the function will be put
/// * `callback` - The function to register
fn op_register_entrypoint(
    state: &mut OpState,
    #[global] callback: v8::Global<v8::Function>,
) -> Result<(), Error> {
    state.put(callback);
    Ok(())
}

#[op2]
#[serde]
fn call_registered_function(
    #[string] name: String,
    #[serde] args: Vec<serde_json::Value>,
    state: &mut OpState,
) -> Result<serde_json::Value, Error> {
    if state.has::<HashMap<String, RsFunction>>() {
        let table = state.borrow_mut::<HashMap<String, RsFunction>>();
        if let Some(callback) = table.get(&name) {
            return callback(&args, state);
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
    if state.has::<HashMap<String, Box<RsAsyncFunction>>>() {
        let table = state.borrow_mut::<HashMap<String, Box<RsAsyncFunction>>>();
        if let Some(callback) = table.get(&name) {
            return callback(args);
        }
    }

    Box::pin(std::future::ready(Err(Error::ValueNotCallable(name))))
}

extension!(
    rustyscript,
    ops = [op_register_entrypoint, call_registered_function, call_registered_function_async],
    esm_entry_point = "ext:rustyscript/rustyscript.js",
    esm = [ dir "src/ext/rustyscript", "rustyscript.js" ],
);

pub fn extensions() -> Vec<Extension> {
    vec![rustyscript::init_ops_and_esm()]
}

pub fn snapshot_extensions() -> Vec<Extension> {
    vec![rustyscript::init_ops()]
}
