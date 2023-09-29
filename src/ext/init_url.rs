use deno_core::extension;

extension!(
    init_url,
    esm_entry_point = "ext:init_url/init_url.js",
    esm = [ dir "src/ext", "init_url.js" ],
);
