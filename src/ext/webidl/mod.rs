use deno_core::extension;

use crate::ext::ExtensionList;

extension!(
    webidl,
    deps = [rustyscript],
    esm_entry_point = "ext:webidl/init_webidl.js",
    esm = [ dir "src/ext/webidl", "init_webidl.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    extensions.extend([deno_webidl::deno_webidl::init(), webidl::init()]);
}
