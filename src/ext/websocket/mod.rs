use deno_core::{extension, Extension};
use deno_core::error::AnyError;
use deno_core::url::Url;
use deno_websocket::WebSocketPermissions;
use crate::ext::web::Permissions;

extension!(
    init_websocket,
    deps = [rustyscript],
    esm_entry_point = "ext:init_websocket/init_websocket.js",
    esm = [ dir "src/ext/websocket", "init_websocket.js" ],
);

impl WebSocketPermissions for Permissions {
    fn check_net_url(&mut self, _url: &Url, _api_name: &str) -> Result<(), AnyError> {
        Ok(())
    }
}

pub fn extensions(user_agent: String) -> Vec<Extension> {
    vec![
        deno_websocket::deno_websocket::init_ops_and_esm::<Permissions>(user_agent, None, None),
        init_websocket::init_ops_and_esm(),
    ]
}

pub fn snapshot_extensions(user_agent: String) -> Vec<Extension> {
    vec![
        deno_websocket::deno_websocket::init_ops::<Permissions>(user_agent, None, None),
        init_websocket::init_ops(),
    ]
}
