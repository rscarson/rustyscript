use std::{borrow::Cow, path::Path, sync::Arc};

use deno_core::{extension, Extension};
use deno_node::NodePermissions;
use deno_permissions::{CheckedPath, PermissionCheckError, PermissionDeniedError};
use deno_resolver::npm::DenoInNpmPackageChecker;
use resolvers::{RustyNpmPackageFolderResolver, RustyResolver};
use sys_traits::impls::RealSys;

use super::{
    web::{PermissionsContainer, SystemsPermissionKind},
    ExtensionTrait,
};

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
        deno_node::deno_node::init::<
            PermissionsContainer,
            DenoInNpmPackageChecker,
            RustyNpmPackageFolderResolver,
            RealSys,
        >(Some(resolver.init_services()), resolver.filesystem())
    }
}

pub fn extensions(resolver: Arc<RustyResolver>, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_node::deno_node::build(resolver, is_snapshot),
        init_node::build((), is_snapshot),
    ]
}

impl NodePermissions for PermissionsContainer {
    fn check_net(
        &mut self,
        host: (&str, Option<u16>),
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        self.0.check_host(host.0, host.1, api_name)?;
        Ok(())
    }

    fn check_open<'a>(
        &mut self,
        path: Cow<'a, Path>,
        open_access: deno_permissions::OpenAccessKind,
        api_name: Option<&str>,
    ) -> Result<deno_permissions::CheckedPath<'a>, PermissionCheckError> {
        let read = open_access.is_read();
        let write = open_access.is_write();

        let p = self
            .0
            .check_open(true, read, write, path, api_name.unwrap_or_default())
            .ok_or(PermissionCheckError::PermissionDenied(
                PermissionDeniedError {
                    access: api_name.unwrap_or_default().to_string(),
                    name: "open",
                },
            ))?;

        Ok(CheckedPath::unsafe_new(p))
    }

    fn check_net_url(
        &mut self,
        url: &reqwest::Url,
        api_name: &str,
    ) -> std::result::Result<(), PermissionCheckError> {
        self.0.check_url(url, api_name)?;
        Ok(())
    }

    fn query_read_all(&mut self) -> bool {
        self.0.check_read_all(None).is_ok()
    }

    fn check_sys(&mut self, kind: &str, api_name: &str) -> Result<(), PermissionCheckError> {
        let kind = SystemsPermissionKind::new(kind);
        self.0.check_sys(kind, api_name)?;
        Ok(())
    }
}
