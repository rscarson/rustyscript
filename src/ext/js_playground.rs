use deno_core::{ v8, op2, extension, OpState };
use crate::error::Error;

#[op2]
/// Registers a JS function with the runtime as being the entrypoint for the script
    ///
    /// # Arguments
    /// * `state` - The runtime's state, into which the function will be put
    /// * `callback` - The function to register
fn op_register_entrypoint(state: &mut OpState, #[global] callback: v8::Global<v8::Function>) -> Result<(), Error> {
    state.put(callback);
    Ok(())
}

extension!(
    js_playground,
    ops = [op_register_entrypoint],
    esm_entry_point = "ext:js_playground/js_playground.js",
    esm = [ dir "src/ext", "js_playground.js" ],
);