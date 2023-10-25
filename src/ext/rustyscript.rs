use crate::error::Error;
use deno_core::{extension, op2, v8, OpState};

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

extension!(
    rustyscript,
    ops = [op_register_entrypoint],
    esm_entry_point = "ext:rustyscript/rustyscript.js",
    esm = [ dir "src/ext", "rustyscript.js" ],
    state = |state| state.put(super::Permissions{})
);
