///
/// This example shows the definition of a simple extension exposing one op
/// Also see example_extension.js
///
/// Extensions like the one below allow you to let JS code call functions
/// in rust
///
/// Extensions consist of a set of #[op2] functions, an extension! macro,
/// and one or more optional JS modules.
///
///
use js_playground::deno_core::{extension, op2};

#[op2]
#[bigint]
fn op_add_example(#[bigint] a: i64, #[bigint] b: i64) -> i64 {
    a + b
}

extension!(
    example_extension,
    ops = [op_add_example],
    esm_entry_point = "ext:example_extension/example_extension.js",
    esm = [ dir "examples/ext", "example_extension.js" ],
);
