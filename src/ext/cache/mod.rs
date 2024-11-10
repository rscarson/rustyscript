use super::ExtensionTrait;
use deno_core::{extension, Extension};

extension!(
    init_cache,
    deps = [rustyscript],
    esm_entry_point = "ext:init_cache/init_cache.js",
    esm = [ dir "src/ext/cache", "init_cache.js" ],
);
impl ExtensionTrait<()> for init_cache {
    fn init((): ()) -> Extension {
        init_cache::init_ops_and_esm()
    }
}
impl ExtensionTrait<Option<deno_cache::CreateCache<deno_cache::SqliteBackedCache>>>
    for deno_cache::deno_cache
{
    fn init(options: Option<deno_cache::CreateCache<deno_cache::SqliteBackedCache>>) -> Extension {
        deno_cache::deno_cache::init_ops_and_esm::<deno_cache::SqliteBackedCache>(options)
    }
}

pub fn extensions(
    options: Option<deno_cache::CreateCache<deno_cache::SqliteBackedCache>>,
    is_snapshot: bool,
) -> Vec<Extension> {
    vec![
        deno_cache::deno_cache::build(options, is_snapshot),
        init_cache::build((), is_snapshot),
    ]
}
