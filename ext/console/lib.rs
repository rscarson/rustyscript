use deno_core::{extension, Extension};
extension!(
    init_console,
    deps = [rustyscript],
    esm_entry_point = "ext:init_console/init_console.js",
    esm = [ dir ".", "init_console.js" ],
);

pub fn extensions() -> Vec<Extension> {
    vec![
        deno_console::deno_console::init_ops_and_esm(),
        init_console::init_ops_and_esm(),
    ]
}
