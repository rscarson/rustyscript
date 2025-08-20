use deno_core::extension;

use crate::ext::ExtensionList;

mod http_runtime;

extension!(
    http,
    deps = [rustyscript],
    esm_entry_point = "ext:http/init_http.js",
    esm = [ dir "src/ext/http", "init_http.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    let http_options = deno_http::Options {
        http2_builder_hook: None,
        http1_builder_hook: None,
        no_legacy_abort: false,
    };
    extensions.extend([
        http_runtime::deno_http_runtime::init(),
        deno_http::deno_http::init(http_options),
        http::init(),
    ]);
}
