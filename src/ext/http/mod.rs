use super::ExtensionTrait;
use deno_core::{extension, Extension};

mod http_runtime;
use http_runtime::deno_http_runtime;
impl ExtensionTrait<()> for deno_http_runtime {
    fn init((): ()) -> Extension {
        deno_http_runtime::init_ops_and_esm()
    }
}

extension!(
    init_http,
    deps = [rustyscript],
    esm_entry_point = "ext:init_http/init_http.js",
    esm = [ dir "src/ext/http", "init_http.js" ],
);
impl ExtensionTrait<()> for init_http {
    fn init((): ()) -> Extension {
        init_http::init_ops_and_esm()
    }
}
impl ExtensionTrait<()> for deno_http::deno_http {
    fn init((): ()) -> Extension {
        deno_http::deno_http::init_ops_and_esm(deno_http::Options {
            http2_builder_hook: None,
            http1_builder_hook: None,
        })
    }
}

pub fn extensions((): (), is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_http_runtime::build((), is_snapshot),
        deno_http::deno_http::build((), is_snapshot),
        init_http::build((), is_snapshot),
    ]
}
