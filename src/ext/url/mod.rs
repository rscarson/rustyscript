use super::ExtensionTrait;
use deno_core::{extension, Extension};

extension!(
    init_url,
    deps = [rustyscript],
    esm_entry_point = "ext:init_url/init_url.js",
    esm = [ dir "src/ext/url", "init_url.js" ],
);
impl ExtensionTrait<()> for init_url {
    fn init((): ()) -> Extension {
        init_url::init()
    }
}
impl ExtensionTrait<()> for deno_url::deno_url {
    fn init((): ()) -> Extension {
        deno_url::deno_url::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_url::deno_url::build((), is_snapshot),
        init_url::build((), is_snapshot),
    ]
}
