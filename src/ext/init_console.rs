use deno_core::extension;

extension!(
    init_console,
    esm_entry_point = "ext:init_console/init_console.js",
    esm = [ dir "src/ext", "init_console.js" ],
);