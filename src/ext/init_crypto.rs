use deno_core::extension;

extension!(
    init_crypto,
    esm_entry_point = "ext:init_crypto/init_crypto.js",
    esm = [ dir "src/ext", "init_crypto.js" ],
);
