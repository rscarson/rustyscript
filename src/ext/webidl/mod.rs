use deno_core::{extension, Extension};

use super::ExtensionTrait;

extension!(
    init_webidl,
    deps = [rustyscript],
    esm_entry_point = "ext:init_webidl/init_webidl.js",
    esm = [ dir "src/ext/webidl", "init_webidl.js" ],
);
impl ExtensionTrait<()> for init_webidl {
    fn init((): ()) -> Extension {
        init_webidl::init()
    }
}
impl ExtensionTrait<()> for deno_webidl::deno_webidl {
    fn init((): ()) -> Extension {
        deno_webidl::deno_webidl::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_webidl::deno_webidl::build((), is_snapshot),
        init_webidl::build((), is_snapshot),
    ]
}
