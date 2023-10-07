use deno_core::extension;

extension!(
    init_web,
    esm_entry_point = "ext:init_web/init_web.js",
    esm = [ dir "src/ext", "init_web.js" ],
);
