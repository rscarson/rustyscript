use deno_core::extension;

extension!(
    init_fetch,
    esm_entry_point = "ext:init_fetch/init_fetch.js",
    esm = [ dir "src/ext", "init_fetch.js" ],
);
