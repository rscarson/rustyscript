use deno_core::{extension, Extension};
extension!(
    init_crypto,
    deps = [rustyscript],
    esm_entry_point = "ext:init_crypto/init_crypto.js",
    esm = [ dir "src/ext/crypto", "init_crypto.js" ],
);

pub fn extensions(seed: Option<u64>) -> Vec<Extension> {
    vec![
        deno_crypto::deno_crypto::init_ops_and_esm(seed),
        init_crypto::init_ops_and_esm(),
    ]
}

pub fn snapshot_extensions(seed: Option<u64>) -> Vec<Extension> {
    vec![
        deno_crypto::deno_crypto::init_ops(seed),
        init_crypto::init_ops(),
    ]
}
