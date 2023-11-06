use deno_core::{extension, Extension};

extension!(
    init_url,
    deps = [rustyscript],
    esm_entry_point = "ext:init_url/init_url.js",
    esm = [ dir ".", "init_url.js" ],
);

pub fn extensions() -> Vec<Extension> {
    vec![
        deno_url::deno_url::init_ops_and_esm(),
        init_url::init_ops_and_esm(),
    ]
}
