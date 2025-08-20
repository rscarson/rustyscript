use deno_core::extension;

use crate::ext::ExtensionList;

extension!(
    cache,
    deps = [rustyscript],
    esm_entry_point = "ext:cache/init_cache.js",
    esm = [ dir "src/ext/cache", "init_cache.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    extensions.extend([
        deno_cache::deno_cache::init(options.cache.clone()),
        cache::init(),
    ]);
}
