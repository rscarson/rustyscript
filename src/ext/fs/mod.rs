use super::{web::PermissionsContainer, ExtensionTrait};
use deno_core::{extension, Extension};
use deno_fs::FileSystemRc;
use deno_permissions::PermissionCheckError;

extension!(
    init_fs,
    deps = [rustyscript],
    esm_entry_point = "ext:init_fs/init_fs.js",
    esm = [ dir "src/ext/fs", "init_fs.js" ],
);
impl ExtensionTrait<()> for init_fs {
    fn init((): ()) -> Extension {
        init_fs::init()
    }
}
impl ExtensionTrait<FileSystemRc> for deno_fs::deno_fs {
    fn init(fs: FileSystemRc) -> Extension {
        deno_fs::deno_fs::init::<PermissionsContainer>(fs)
    }
}

pub fn extensions(fs: FileSystemRc, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_fs::deno_fs::build(fs, is_snapshot),
        init_fs::build((), is_snapshot),
    ]
}

impl deno_fs::FsPermissions for PermissionsContainer {
    fn check_open_blind<'a>(
        &self,
        path: std::borrow::Cow<'a, std::path::Path>,
        _access_kind: deno_permissions::OpenAccessKind,
        _display: &str,
        _api_name: &str,
    ) -> Result<deno_permissions::CheckedPath<'a>, PermissionCheckError> {
        // Default implementation - allow all opens
        Ok(deno_permissions::CheckedPath::unsafe_new(path))
    }

    fn check_open<'a>(
        &self,
        path: std::borrow::Cow<'a, std::path::Path>,
        access_kind: deno_permissions::OpenAccessKind,
        api_name: &str,
    ) -> Result<deno_permissions::CheckedPath<'a>, PermissionCheckError> {
        // Default implementation - allow all opens
        Ok(deno_permissions::CheckedPath::unsafe_new(path))
    }

    fn check_read_all(&self, api_name: &str) -> Result<(), PermissionCheckError> {
        // Default implementation - allow all reads
        Ok(())
    }

    fn check_write_partial<'a>(
        &self,
        path: std::borrow::Cow<'a, std::path::Path>,
        api_name: &str,
    ) -> Result<deno_permissions::CheckedPath<'a>, PermissionCheckError> {
        // Default implementation - allow all writes
        Ok(deno_permissions::CheckedPath::unsafe_new(path))
    }

    fn check_write_all(&self, api_name: &str) -> Result<(), PermissionCheckError> {
        // Default implementation - allow all writes
        Ok(())
    }
}
