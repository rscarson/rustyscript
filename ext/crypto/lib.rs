use deno_core::{extension, Extension};
extension!(
    init_crypto,
    deps = [rustyscript],
    esm_entry_point = "ext:init_crypto/init_crypto.js",
    esm = [ dir ".", "init_crypto.js" ],
);

pub fn extensions() -> Vec<Extension> {
    vec![
        deno_crypto::deno_crypto::init_ops_and_esm(None),
        init_crypto::init_ops_and_esm(),
    ]
}
