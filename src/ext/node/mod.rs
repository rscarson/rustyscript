use std::{borrow::Cow, path::Path};

use deno_core::extension;
use deno_node::NodePermissions;
use deno_permissions::{CheckedPath, PermissionCheckError, PermissionDeniedError};
use deno_resolver::npm::DenoInNpmPackageChecker;
use resolvers::RustyNpmPackageFolderResolver;
use sys_traits::impls::RealSys;

use super::web::{PermissionsContainer, SystemsPermissionKind};
use crate::ext::ExtensionList;

mod cjs_translator;
pub mod resolvers;
pub use cjs_translator::NodeCodeTranslator;

extension!(
    node,
    deps = [rustyscript],
    esm_entry_point = "ext:node/init_node.js",
    esm = [ dir "src/ext/node", "init_node.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    let resolver = options.node_resolver.clone();
    extensions.extend([
        deno_node::deno_node::init::<
            PermissionsContainer,
            DenoInNpmPackageChecker,
            RustyNpmPackageFolderResolver,
            RealSys,
        >(Some(resolver.init_services()), resolver.filesystem()),
        node::init(),
    ]);
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
