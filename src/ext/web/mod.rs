use deno_core::{extension, Extension, ModuleSpecifier};
use std::{rc::Rc, sync::Arc};

#[derive(Clone)]
pub struct Permissions;

impl deno_web::TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        true
    }
}

impl deno_fetch::FetchPermissions for Permissions {
    fn check_net_url(
        &mut self,
        _url: &deno_core::url::Url,
        _api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }

    fn check_read(
        &mut self,
        _p: &std::path::Path,
        _api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }
}

impl deno_net::NetPermissions for Permissions {
    fn check_net<T: AsRef<str>>(
        &mut self,
        _host: &(T, Option<u16>),
        _api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }

    fn check_read(
        &mut self,
        _p: &std::path::Path,
        _api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }

    fn check_write(
        &mut self,
        _p: &std::path::Path,
        _api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }
}

extension!(
    init_web,
    deps = [rustyscript],
    esm_entry_point = "ext:init_web/init_web.js",
    esm = [ dir "src/ext/web", "init_web.js" ],
    state = |state| state.put(Permissions{})
);

extension!(
    init_fetch,
    deps = [rustyscript],
    esm_entry_point = "ext:init_fetch/init_fetch.js",
    esm = [ dir "src/ext/web", "init_fetch.js" ],
    state = |state| state.put(Permissions{})
);

extension!(
    init_net,
    deps = [rustyscript],
    esm_entry_point = "ext:init_net/init_net.js",
    esm = [ dir "src/ext/web", "init_net.js" ],
);

/// Options for configuring the web related extensions
#[derive(Clone)]
pub struct WebOptions {
    /// Base URL for some deno_web OPs
    pub base_url: Option<ModuleSpecifier>,

    /// User agent to use for fetch
    pub user_agent: String,

    /// Root certificate store for TLS connections for fetches and network OPs
    pub root_cert_store_provider: Option<Arc<dyn deno_tls::RootCertStoreProvider>>,

    /// Proxy for fetch
    pub proxy: Option<deno_tls::Proxy>,

    /// Request builder hook for fetch
    pub request_builder_hook: Option<
        fn(
            _: deno_fetch::reqwest::RequestBuilder,
        ) -> Result<deno_fetch::reqwest::RequestBuilder, deno_core::error::AnyError>,
    >,

    /// If true, fetches and network OPs will ignore SSL errors
    pub unsafely_ignore_certificate_errors: Option<Vec<String>>,

    /// Client certificate and key for fetch
    pub client_cert_chain_and_key: deno_tls::TlsKeys,

    /// File fetch handler for fetch
    pub file_fetch_handler: Rc<dyn deno_fetch::FetchHandler>,
}

impl Default for WebOptions {
    fn default() -> Self {
        Self {
            base_url: None,
            user_agent: "".to_string(),
            root_cert_store_provider: None,
            proxy: None,
            request_builder_hook: None,
            unsafely_ignore_certificate_errors: None,
            client_cert_chain_and_key: deno_tls::TlsKeys::Null,
            file_fetch_handler: Rc::new(deno_fetch::DefaultFileFetchHandler),
        }
    }
}

pub fn extensions(options: WebOptions) -> Vec<Extension> {
    vec![
        deno_web::deno_web::init_ops_and_esm::<Permissions>(
            Default::default(),
            options.base_url.clone(),
        ),
        deno_net::deno_net::init_ops_and_esm::<Permissions>(
            options.root_cert_store_provider.clone(),
            options.unsafely_ignore_certificate_errors.clone(),
        ),
        deno_fetch::deno_fetch::init_ops_and_esm::<Permissions>(deno_fetch::Options {
            user_agent: options.user_agent,
            root_cert_store_provider: options.root_cert_store_provider,
            proxy: options.proxy,
            request_builder_hook: options.request_builder_hook,
            unsafely_ignore_certificate_errors: options.unsafely_ignore_certificate_errors,
            client_cert_chain_and_key: options.client_cert_chain_and_key,
            file_fetch_handler: options.file_fetch_handler,
        }),
        init_web::init_ops_and_esm(),
        init_fetch::init_ops_and_esm(),
        init_net::init_ops_and_esm(),
    ]
}

pub fn snapshot_extensions(options: WebOptions) -> Vec<Extension> {
    vec![
        deno_web::deno_web::init_ops::<Permissions>(Default::default(), options.base_url.clone()),
        deno_net::deno_net::init_ops::<Permissions>(
            options.root_cert_store_provider.clone(),
            options.unsafely_ignore_certificate_errors.clone(),
        ),
        deno_fetch::deno_fetch::init_ops::<Permissions>(deno_fetch::Options {
            user_agent: options.user_agent,
            root_cert_store_provider: options.root_cert_store_provider,
            proxy: options.proxy,
            request_builder_hook: options.request_builder_hook,
            unsafely_ignore_certificate_errors: options.unsafely_ignore_certificate_errors,
            client_cert_chain_and_key: options.client_cert_chain_and_key,
            file_fetch_handler: options.file_fetch_handler,
        }),
        init_web::init_ops(),
        init_fetch::init_ops(),
        init_net::init_ops(),
    ]
}
