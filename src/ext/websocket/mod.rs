use crate::WebOptions;
use deno_core::error::AnyError;
use deno_core::url::Url;
use deno_core::{extension, Extension};
use deno_websocket::WebSocketPermissions;

#[derive(Clone, Default)]
struct Permissions;
impl WebSocketPermissions for Permissions {
    fn check_net_url(&mut self, _url: &Url, _api_name: &str) -> Result<(), AnyError> {
        Ok(())
    }
}

extension!(
    init_websocket,
    deps = [rustyscript],
    esm_entry_point = "ext:init_websocket/init_websocket.js",
    esm = [ dir "src/ext/websocket", "init_websocket.js" ],
    state = |state| state.put(Permissions)
);

pub fn extensions(options: WebOptions) -> Vec<Extension> {
    vec![
        deno_websocket::deno_websocket::init_ops_and_esm::<Permissions>(
            options.user_agent,
            options.root_cert_store_provider,
            options.unsafely_ignore_certificate_errors,
        ),
        init_websocket::init_ops_and_esm(),
    ]
}

pub fn snapshot_extensions(options: WebOptions) -> Vec<Extension> {
    vec![
        deno_websocket::deno_websocket::init_ops::<Permissions>(
            options.user_agent,
            options.root_cert_store_provider,
            options.unsafely_ignore_certificate_errors,
        ),
        init_websocket::init_ops(),
    ]
}
