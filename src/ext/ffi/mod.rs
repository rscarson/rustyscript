use std::{borrow::Cow, path::Path};

use deno_core::extension;

use super::web::PermissionsContainer;
use crate::ext::ExtensionList;

extension!(
    ffi,
    deps = [rustyscript],
    esm_entry_point = "ext:ffi/init_ffi.js",
    esm = [ dir "src/ext/ffi", "init_ffi.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    extensions.extend([
        deno_ffi::deno_ffi::init::<PermissionsContainer>(options.ffi_addon_loader.clone()),
        ffi::init(),
    ]);
}

impl deno_ffi::FfiPermissions for PermissionsContainer {
    fn check_partial_no_path(&mut self) -> Result<(), deno_permissions::PermissionCheckError> {
        self.0.check_exec()?;
        Ok(())
    }

    fn check_partial_with_path<'a>(
        &mut self,
        path: Cow<'a, Path>,
    ) -> Result<Cow<'a, Path>, deno_permissions::PermissionCheckError> {
        self.check_partial_no_path()?;
        let p = self.0.check_read(path, None)?;
        Ok(p)
    }
}
