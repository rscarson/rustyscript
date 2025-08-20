use deno_core::extension;

use crate::ext::ExtensionList;

extension!(
    url,
    deps = [rustyscript],
    esm_entry_point = "ext:url/init_url.js",
    esm = [ dir "src/ext/url", "init_url.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    extensions.extend([deno_url::deno_url::init(), url::init()]);
}
