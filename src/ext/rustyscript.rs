use std::collections::HashMap;

use crate::{error::Error, FunctionArguments, RsFunction};
use deno_core::{extension, op2, serde_json, v8, OpState};

fn call_rs_fn(
    name: &str,
    args: &FunctionArguments,
    state: &mut OpState,
) -> Result<serde_json::Value, Error> {
    if state.has::<HashMap<String, RsFunction>>() {
        let table = state.borrow_mut::<HashMap<
            String,
            fn(&FunctionArguments, &mut OpState) -> Result<serde_json::Value, Error>,
        >>();
        if let Some(callback) = table.get(name) {
            return callback(args, state);
        }
    }

    Err(Error::ValueNotCallable(name.to_string()))
}

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
    call_rs_fn(&name, args.as_slice(), state)
}

extension!(
    rustyscript,
    ops = [op_register_entrypoint, call_registered_function],
    esm_entry_point = "ext:rustyscript/rustyscript.js",
    esm = [ dir "src/ext", "rustyscript.js" ],
    state = |state| state.put(super::Permissions{})
);
