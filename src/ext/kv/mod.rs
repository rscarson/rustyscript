use std::borrow::Cow;

use deno_core::extension;
use deno_kv::{remote::RemoteDbHandlerPermissions, sqlite::SqliteDbHandlerPermissions};
use deno_permissions::{CheckedPath, PermissionCheckError, PermissionDeniedError};

use super::web::PermissionsContainer;
use crate::ext::ExtensionList;

mod backend;
pub use backend::{KvConfig, KvStore};

extension!(
    kv,
    deps = [rustyscript],
    esm_entry_point = "ext:kv/init_kv.js",
    esm = [ dir "src/ext/kv", "init_kv.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    extensions.extend([
        deno_kv::deno_kv::init(options.kv_store.handler(), options.kv_store.config()),
        kv::init(),
    ]);
}

impl SqliteDbHandlerPermissions for PermissionsContainer {
    fn check_open<'a>(
        &mut self,
        path: Cow<'a, std::path::Path>,
        open_access: deno_permissions::OpenAccessKind,
        api_name: &str,
    ) -> Result<CheckedPath<'a>, deno_permissions::PermissionCheckError> {
        let read = open_access.is_read();
        let write = open_access.is_write();

        let p = self.0.check_open(true, read, write, path, api_name).ok_or(
            PermissionCheckError::PermissionDenied(PermissionDeniedError {
                access: api_name.to_string(),
                name: "open",
            }),
        )?;

        Ok(CheckedPath::unsafe_new(p))
    }
}

impl RemoteDbHandlerPermissions for PermissionsContainer {
    fn check_env(&mut self, var: &str) -> Result<(), deno_permissions::PermissionCheckError> {
        self.0.check_env(var)?;
        Ok(())
    }

    fn check_net_url(
        &mut self,
        url: &reqwest::Url,
        api_name: &str,
    ) -> Result<(), deno_permissions::PermissionCheckError> {
        self.0.check_url(url, api_name)?;
        Ok(())
    }
}
