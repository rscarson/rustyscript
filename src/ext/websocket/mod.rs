use crate::ext::web::PermissionsContainer;
use crate::WebOptions;
use deno_core::error::AnyError;
use deno_core::url::Url;
use deno_core::{extension, Extension};

#[derive(Clone, Default)]
struct Permissions;
impl deno_websocket::WebSocketPermissions for PermissionsContainer {
    fn check_net_url(&mut self, url: &Url, api_name: &str) -> Result<(), AnyError> {
        self.0.check_url(url, api_name)
    }
}

extension!(
    init_websocket,
    deps = [rustyscript],
    esm_entry_point = "ext:init_websocket/init_websocket.js",
    esm = [ dir "src/ext/websocket", "init_websocket.js" ],
);

pub fn extensions(options: WebOptions) -> Vec<Extension> {
    vec![
        deno_websocket::deno_websocket::init_ops_and_esm::<PermissionsContainer>(
            options.user_agent,
            options.root_cert_store_provider,
            options.unsafely_ignore_certificate_errors,
        ),
        init_websocket::init_ops_and_esm(),
    ]
}

pub fn snapshot_extensions(options: WebOptions) -> Vec<Extension> {
    vec![
        deno_websocket::deno_websocket::init_ops::<PermissionsContainer>(
            options.user_agent,
            options.root_cert_store_provider,
            options.unsafely_ignore_certificate_errors,
        ),
        init_websocket::init_ops(),
    ]
}
