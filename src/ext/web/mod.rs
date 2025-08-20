use std::sync::Arc;

use deno_core::extension;

use crate::ext::ExtensionList;

mod options;
pub use options::WebOptions;

mod permissions;
pub(crate) use permissions::PermissionsContainer;
pub use permissions::{
    AllowlistWebPermissions, CheckedPath, DefaultWebPermissions, PermissionCheckError,
    PermissionDeniedError, SystemsPermissionKind, WebPermissions,
};

/// Stub for a node op `deno_net` expects to find
/// We return None to show no cert available
#[deno_core::op2]
#[serde]
pub fn op_tls_peer_certificate(
    #[smi] _rid: u32,
    _detailed: bool,
) -> Option<deno_core::serde_json::Value> {
    None
}

extension!(
    fetch,
    deps = [rustyscript],
    esm_entry_point = "ext:fetch/init_fetch.js",
    esm = [ dir "src/ext/web", "init_fetch.js" ],
);

#[cfg(not(feature = "node_experimental"))]
extension!(
    net,
    deps = [rustyscript],
    ops = [op_tls_peer_certificate],
    esm_entry_point = "ext:net/init_net.js",
    esm = [ dir "src/ext/web", "init_net.js" ],
);

#[cfg(feature = "node_experimental")]
extension!(
    net,
    deps = [rustyscript],
    esm_entry_point = "ext:net/init_net.js",
    esm = [ dir "src/ext/web", "init_net.js" ],
);

extension!(
    telemetry,
    deps = [rustyscript],
    esm_entry_point = "ext:telemetry/init_telemetry.js",
    esm = [ dir "src/ext/web", "init_telemetry.js" ],
);

extension!(
    web,
    deps = [rustyscript],
    esm_entry_point = "ext:web/init_web.js",
    esm = [ dir "src/ext/web", "init_web.js", "init_errors.js" ],
    options = {
        permissions: Arc<dyn WebPermissions>
    },
    state = |state, config| state.put(PermissionsContainer(config.permissions)),
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    let web_permissions = options.web.permissions.clone();
    let blob_store = options.web.blob_store.clone();
    let base_url = options.web.base_url.clone();
    let provider = options.web.root_cert_store_provider.clone();
    let unsafe_ssl = options.web.unsafely_ignore_certificate_errors.clone();

    let _ = rustls::crypto::CryptoProvider::install_default(
        rustls::crypto::aws_lc_rs::default_provider(),
    ); // Failure means already done for us

    let fetch_options = deno_fetch::Options {
        user_agent: options.web.user_agent.clone(),
        root_cert_store_provider: options.web.root_cert_store_provider.clone(),
        proxy: options.web.proxy.clone(),
        request_builder_hook: options.web.request_builder_hook,
        unsafely_ignore_certificate_errors: options.web.unsafely_ignore_certificate_errors.clone(),
        client_cert_chain_and_key: options.web.client_cert_chain_and_key.clone(),
        file_fetch_handler: options.web.file_fetch_handler.clone(),
        client_builder_hook: options.web.client_builder_hook,
        resolver: options.web.resolver.clone(),
    };

    extensions.extend([
        deno_web::deno_web::init::<PermissionsContainer>(blob_store, base_url),
        deno_telemetry::deno_telemetry::init(),
        deno_net::deno_net::init::<PermissionsContainer>(provider, unsafe_ssl),
        deno_fetch::deno_fetch::init::<PermissionsContainer>(fetch_options),
        deno_tls::deno_tls::init(),
        //
        web::init(web_permissions),
        telemetry::init(),
        //
        #[cfg(not(feature = "node_experimental"))]
        net::init(),
        //
        fetch::init(),
    ]);
}
