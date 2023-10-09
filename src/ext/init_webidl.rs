use deno_core::extension;

extension!(
    init_webidl,
    esm_entry_point = "ext:init_webidl/init_webidl.js",
    esm = [ dir "src/ext", "init_webidl.js" ],
);
