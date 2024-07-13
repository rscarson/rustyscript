use std::sync::Arc;
use deno_core::{extension, Extension};
use deno_core::error::AnyError;
use deno_core::url::Url;
use deno_tls::RootCertStoreProvider;
use deno_websocket::WebSocketPermissions;
use crate::ext::web::Permissions;
use crate::WebOptions;

extension!(
    init_websocket,
    deps = [rustyscript],
    esm_entry_point = "ext:init_websocket/init_websocket.js",
    esm = [ dir "src/ext/websocket", "init_websocket.js" ],
);

pub struct WebSocketOptions {
    pub user_agent: String,
    pub root_cert_store_provider: Option<Arc<dyn RootCertStoreProvider>>,
    pub unsafely_ignore_certificate_errors: Option<Vec<String>>,
}

impl From<WebOptions> for WebSocketOptions {
    fn from(options: WebOptions) -> Self {
        Self {
            user_agent: options.user_agent,
            root_cert_store_provider: options.root_cert_store_provider,
            unsafely_ignore_certificate_errors: options.unsafely_ignore_certificate_errors,
        }
    }
}

impl WebSocketPermissions for Permissions {
    fn check_net_url(&mut self, _url: &Url, _api_name: &str) -> Result<(), AnyError> {
        Ok(())
    }
}

pub fn extensions(options: WebSocketOptions) -> Vec<Extension> {
    vec![
        deno_websocket::deno_websocket::init_ops_and_esm::<Permissions>(
            options.user_agent,
            options.root_cert_store_provider,
            options.unsafely_ignore_certificate_errors
        ),
        init_websocket::init_ops_and_esm(),
    ]
}

pub fn snapshot_extensions(options: WebSocketOptions) -> Vec<Extension> {
    vec![
        deno_websocket::deno_websocket::init_ops::<Permissions>(
            options.user_agent,
            options.root_cert_store_provider,
            options.unsafely_ignore_certificate_errors
        ),
        init_websocket::init_ops(),
    ]
}
