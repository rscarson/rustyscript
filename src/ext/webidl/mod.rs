use deno_core::{extension, Extension};
extension!(
    init_webidl,
    deps = [rustyscript],
    esm_entry_point = "ext:init_webidl/init_webidl.js",
    esm = [ dir "src/ext/webidl", "init_webidl.js" ],
);

pub fn extensions() -> Vec<Extension> {
    vec![
        deno_webidl::deno_webidl::init_ops_and_esm(),
        init_webidl::init_ops_and_esm(),
    ]
}
