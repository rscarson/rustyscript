use std::borrow::Cow;

use deno_core::extension;

use super::web::PermissionsContainer;
use crate::ext::ExtensionList;

extension!(
    napi,
    deps = [rustyscript],
    esm_entry_point = "ext:napi/init_napi.js",
    esm = [ dir "src/ext/napi", "init_napi.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    extensions.extend([
        deno_napi::deno_napi::init::<PermissionsContainer>(options.ffi_addon_loader.clone()),
        napi::init(),
    ]);
}

impl deno_napi::NapiPermissions for PermissionsContainer {
    fn check<'a>(
        &mut self,
        path: Cow<'a, std::path::Path>,
    ) -> Result<Cow<'a, std::path::Path>, deno_permissions::PermissionCheckError> {
        let p = self.0.check_read(path, None)?;
        Ok(p)
    }
}
