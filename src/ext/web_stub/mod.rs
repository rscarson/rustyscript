//! This module is a stub for the `deno_web` extension.
//! It is used when the `web` feature is disabled.
//!
//! It provides a minimal set of APIs that are required for a few other extensions.
use super::ExtensionTrait;
use deno_core::{extension, Extension};

mod timers;

extension!(
    deno_web,
    ops = [
        timers::op_now, timers::op_defer,
    ],
    esm_entry_point = "ext:deno_web/init_stub.js",
    esm = [ dir "src/ext/web_stub", "init_stub.js", "01_dom_exception.js", "02_timers.js" ],
);
impl ExtensionTrait<()> for deno_web {
    fn init((): ()) -> Extension {
        deno_web::init_ops_and_esm()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_web::build((), is_snapshot)]
}
