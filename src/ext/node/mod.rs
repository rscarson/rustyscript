use super::{
    web::{PermissionsContainer, SystemsPermissionKind},
    ExtensionTrait,
};
use deno_core::{extension, Extension};
use deno_node::NodePermissions;
use deno_permissions::PermissionCheckError;
use std::{path::Path, sync::Arc};

mod cjs_translator;
mod resolvers;
pub use cjs_translator::NodeCodeTranslator;
pub use resolvers::RustyResolver;

extension!(
    init_node,
    deps = [rustyscript],
    esm_entry_point = "ext:init_node/init_node.js",
    esm = [ dir "src/ext/node", "init_node.js" ],
);
impl ExtensionTrait<()> for init_node {
    fn init((): ()) -> Extension {
        init_node::init_ops_and_esm()
    }
}
impl ExtensionTrait<Arc<RustyResolver>> for deno_node::deno_node {
    fn init(resolver: Arc<RustyResolver>) -> Extension {
        deno_node::deno_node::init_ops_and_esm::<PermissionsContainer>(
            Some(resolver.init_services()),
            resolver.filesystem(),
        )
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

    fn check_read(&mut self, path: &str) -> Result<std::path::PathBuf, PermissionCheckError> {
        let p = self.0.check_read(Path::new(path), None)?;
        Ok(p.into_owned())
    }

    fn check_net_url(
        &mut self,
        url: &reqwest::Url,
        api_name: &str,
    ) -> std::result::Result<(), PermissionCheckError> {
        self.0.check_url(url, api_name)?;
        Ok(())
    }

    fn check_read_with_api_name(
        &mut self,
        path: &str,
        api_name: Option<&str>,
    ) -> Result<std::path::PathBuf, PermissionCheckError> {
        let p = self
            .0
            .check_read(Path::new(path), api_name)
            .map(std::borrow::Cow::into_owned)?;
        Ok(p)
    }

    fn check_read_path<'a>(
        &mut self,
        path: &'a std::path::Path,
    ) -> Result<std::borrow::Cow<'a, std::path::Path>, PermissionCheckError> {
        let p = self.0.check_read(path, None)?;
        Ok(p)
    }

    fn query_read_all(&mut self) -> bool {
        self.0.check_read_all(None).is_ok()
    }

    fn check_sys(&mut self, kind: &str, api_name: &str) -> Result<(), PermissionCheckError> {
        let kind = SystemsPermissionKind::new(kind);
        self.0.check_sys(kind, api_name)?;
        Ok(())
    }

    fn check_write_with_api_name(
        &mut self,
        path: &str,
        api_name: Option<&str>,
    ) -> Result<std::path::PathBuf, PermissionCheckError> {
        let p = self
            .0
            .check_write(Path::new(path), api_name)
            .map(std::borrow::Cow::into_owned)?;
        Ok(p)
    }
}
