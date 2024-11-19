use super::ExtensionTrait;
use deno_core::{extension, Extension};

mod cache_backend;
mod memory;
pub use cache_backend::CacheBackend;

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
impl ExtensionTrait<Option<deno_cache::CreateCache<CacheBackend>>> for deno_cache::deno_cache {
    fn init(options: Option<deno_cache::CreateCache<CacheBackend>>) -> Extension {
        deno_cache::deno_cache::init_ops_and_esm::<CacheBackend>(options)
    }
}

pub fn extensions(
    options: Option<deno_cache::CreateCache<CacheBackend>>,
    is_snapshot: bool,
) -> Vec<Extension> {
    vec![
        deno_cache::deno_cache::build(options, is_snapshot),
        init_cache::build((), is_snapshot),
    ]
}
