use deno_core::{extension, Extension};
extension!(
    init_webstorage,
    deps = [rustyscript],
    esm_entry_point = "ext:init_webstorage/init_webstorage.js",
    esm = [ dir "src/ext/webstorage", "init_webstorage.js" ],
);

pub fn extensions(origin_storage_dir: Option<PathBuf>) -> Vec<Extension> {
    vec![
        deno_webstorage::deno_webstorage::init_ops_and_esm(origin_storage_dir),
        init_webstorage::init_ops_and_esm(),
    ]
}

pub fn snapshot_extensions(origin_storage_dir: Option<PathBuf>) -> Vec<Extension> {
    vec![
        deno_webstorage::deno_webstorage::init_ops(origin_storage_dir),
        init_webstorage::init_ops(),
    ]
}
