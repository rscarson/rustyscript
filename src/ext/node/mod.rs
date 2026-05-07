use std::sync::Arc;

use deno_core::{extension, Extension};
use deno_resolver::npm::DenoInNpmPackageChecker;
use resolvers::{RustyNpmPackageFolderResolver, RustyResolver};
use sys_traits::impls::RealSys;

use super::ExtensionTrait;

mod cjs_translator;
pub mod resolvers;
pub use cjs_translator::NodeCodeTranslator;

extension!(
    init_node,
    deps = [rustyscript],
    esm_entry_point = "ext:init_node/init_node.js",
    esm = [ dir "src/ext/node", "init_node.js" ],
);
impl ExtensionTrait<()> for init_node {
    fn init((): ()) -> Extension {
        init_node::init()
    }
}
impl ExtensionTrait<Arc<RustyResolver>> for deno_node::deno_node {
    fn init(resolver: Arc<RustyResolver>) -> Extension {
        deno_node::deno_node::init::<DenoInNpmPackageChecker, RustyNpmPackageFolderResolver, RealSys>(
            Some(resolver.init_services()),
            resolver.filesystem(),
        )
    }
}
impl ExtensionTrait<()> for deno_node_crypto::deno_node_crypto {
    fn init((): ()) -> Extension {
        deno_node_crypto::deno_node_crypto::init()
    }
}
impl ExtensionTrait<()> for deno_node_sqlite::deno_node_sqlite {
    fn init((): ()) -> Extension {
        deno_node_sqlite::deno_node_sqlite::init()
    }
}

pub fn extensions(resolver: Arc<RustyResolver>, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_node_crypto::deno_node_crypto::build((), is_snapshot),
        deno_node_sqlite::deno_node_sqlite::build((), is_snapshot),
        deno_node::deno_node::build(resolver, is_snapshot),
        init_node::build((), is_snapshot),
    ]
}
