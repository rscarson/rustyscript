use deno_core::extension;

use crate::ext::ExtensionList;

extension!(
    webstorage,
    deps = [rustyscript],
    esm_entry_point = "ext:webstorage/init_webstorage.js",
    esm = [ dir "src/ext/webstorage", "init_webstorage.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    extensions.extend([
        deno_webstorage::deno_webstorage::init(options.webstorage_origin_storage_dir.clone()),
        webstorage::init(),
    ]);
}
