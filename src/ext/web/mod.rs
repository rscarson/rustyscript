use super::ExtensionTrait;
use deno_core::{extension, Extension};
use std::sync::Arc;

mod options;
pub use options::WebOptions;

mod permissions;
pub(crate) use permissions::PermissionsContainer;
pub use permissions::{
    AllowlistWebPermissions, DefaultWebPermissions, PermissionDenied, SystemsPermissionKind,
    WebPermissions,
};

extension!(
    init_fetch,
    deps = [rustyscript],
    esm_entry_point = "ext:init_fetch/init_fetch.js",
    esm = [ dir "src/ext/web", "init_fetch.js" ],
);
impl ExtensionTrait<WebOptions> for init_fetch {
    fn init(options: WebOptions) -> Extension {
        init_fetch::init_ops_and_esm()
    }
}
impl ExtensionTrait<WebOptions> for deno_fetch::deno_fetch {
    fn init(options: WebOptions) -> Extension {
        let options = deno_fetch::Options {
            user_agent: options.user_agent.clone(),
            root_cert_store_provider: options.root_cert_store_provider.clone(),
            proxy: options.proxy.clone(),
            request_builder_hook: options.request_builder_hook,
            unsafely_ignore_certificate_errors: options.unsafely_ignore_certificate_errors.clone(),
            client_cert_chain_and_key: options.client_cert_chain_and_key.clone(),
            file_fetch_handler: options.file_fetch_handler.clone(),
            client_builder_hook: options.client_builder_hook,
            resolver: options.resolver.clone(),
        };

        deno_fetch::deno_fetch::init_ops_and_esm::<PermissionsContainer>(options)
    }
}

extension!(
    init_net,
    deps = [rustyscript],
    esm_entry_point = "ext:init_net/init_net.js",
    esm = [ dir "src/ext/web", "init_net.js" ],
);
impl ExtensionTrait<WebOptions> for init_net {
    fn init(options: WebOptions) -> Extension {
        init_net::init_ops_and_esm()
    }
}
impl ExtensionTrait<WebOptions> for deno_net::deno_net {
    fn init(options: WebOptions) -> Extension {
        deno_net::deno_net::init_ops_and_esm::<PermissionsContainer>(
            options.root_cert_store_provider.clone(),
            options.unsafely_ignore_certificate_errors.clone(),
        )
    }
}

extension!(
    init_web,
    deps = [rustyscript],
    esm_entry_point = "ext:init_web/init_web.js",
    esm = [ dir "src/ext/web", "init_web.js", "init_errors.js" ],
    options = {
        permissions: Arc<dyn WebPermissions>
    },
    state = |state, config| state.put(PermissionsContainer(config.permissions)),
);
impl ExtensionTrait<WebOptions> for init_web {
    fn init(options: WebOptions) -> Extension {
        init_web::init_ops_and_esm(options.permissions)
    }
}

impl ExtensionTrait<WebOptions> for deno_web::deno_web {
    fn init(options: WebOptions) -> Extension {
        deno_web::deno_web::init_ops_and_esm::<PermissionsContainer>(
            options.blob_store,
            options.base_url,
        )
    }
}

impl ExtensionTrait<()> for deno_tls::deno_tls {
    fn init((): ()) -> Extension {
        deno_tls::deno_tls::init_ops_and_esm()
    }
}

pub fn extensions(options: WebOptions, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_web::deno_web::build(options.clone(), is_snapshot),
        deno_net::deno_net::build(options.clone(), is_snapshot),
        deno_fetch::deno_fetch::build(options.clone(), is_snapshot),
        deno_tls::deno_tls::build((), is_snapshot),
        init_web::build(options.clone(), is_snapshot),
        init_net::build(options.clone(), is_snapshot),
        init_fetch::build(options, is_snapshot),
    ]
}
