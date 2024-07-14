use deno_core::{extension, Extension};

extension!(
    init_cache,
    deps = [rustyscript],
    esm_entry_point = "ext:init_cache/init_cache.js",
    esm = [ dir "src/ext/cache", "init_cache.js" ],
);

pub fn extensions(
    cache: Option<deno_cache::CreateCache<deno_cache::SqliteBackedCache>>,
) -> Vec<Extension> {
    vec![
        deno_cache::deno_cache::init_ops_and_esm::<deno_cache::SqliteBackedCache>(cache),
        init_cache::init_ops_and_esm(),
    ]
}

pub fn snapshot_extensions(
    cache: Option<deno_cache::CreateCache<deno_cache::SqliteBackedCache>>,
) -> Vec<Extension> {
    vec![
        deno_cache::deno_cache::init_ops::<deno_cache::SqliteBackedCache>(cache),
        init_cache::init_ops(),
    ]
}
