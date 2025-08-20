use deno_core::{extension, url::Url};
use deno_permissions::PermissionCheckError;

use super::web::PermissionsContainer;
use crate::ext::ExtensionList;

impl deno_websocket::WebSocketPermissions for PermissionsContainer {
    fn check_net_url(&mut self, url: &Url, api_name: &str) -> Result<(), PermissionCheckError> {
        self.0.check_url(url, api_name)?;
        Ok(())
    }
}

extension!(
    websocket,
    deps = [rustyscript],
    esm_entry_point = "ext:websocket/init_websocket.js",
    esm = [ dir "src/ext/websocket", "init_websocket.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    let user_agent = options.web.user_agent.clone();
    let store = options.web.root_cert_store_provider.clone();
    let unsafe_ssl = options.web.unsafely_ignore_certificate_errors.clone();

    extensions.extend([
        deno_websocket::deno_websocket::init::<PermissionsContainer>(user_agent, store, unsafe_ssl),
        websocket::init(),
    ]);
}
