use super::{web::PermissionsContainer, ExtensionTrait};
use deno_core::{extension, Extension};

extension!(
    init_ffi,
    deps = [rustyscript],
    esm_entry_point = "ext:init_ffi/init_ffi.js",
    esm = [ dir "src/ext/ffi", "init_ffi.js" ],
);
impl ExtensionTrait<()> for init_ffi {
    fn init((): ()) -> Extension {
        init_ffi::init()
    }
}
impl ExtensionTrait<()> for deno_ffi::deno_ffi {
    fn init((): ()) -> Extension {
        deno_ffi::deno_ffi::init::<PermissionsContainer>()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_ffi::deno_ffi::build((), is_snapshot),
        init_ffi::build((), is_snapshot),
    ]
}

impl deno_ffi::FfiPermissions for PermissionsContainer {
    fn check_partial_no_path(&mut self) -> Result<(), deno_permissions::PermissionCheckError> {
        self.0.check_exec()?;
        Ok(())
    }

    fn check_partial_with_path(
        &mut self,
        path: &str,
    ) -> Result<std::path::PathBuf, deno_permissions::PermissionCheckError> {
        self.check_partial_no_path()?;
        let p = self.0.check_read(std::path::Path::new(path), None)?;
        Ok(p.to_path_buf())
    }
}
