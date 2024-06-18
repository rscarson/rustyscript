use deno_core::{extension, Extension};

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
#[derive(Debug, Default)]
pub struct WebOptions {
    /// Base URL for some deno_web OPs
    pub base_url: Option<ModuleSpecifier>,

    /// User agent to use for fetch
    pub user_agent: String,

    /// Root certificate store for TLS connections for fetches and network OPs
    pub root_cert_store_provider: Option<Arc<dyn RootCertStoreProvider>>,

    /// Proxy for fetch
    pub proxy: Option<Proxy>,

    /// Request builder hook for fetch
    pub request_builder_hook: Option<fn(_: RequestBuilder) -> Result<RequestBuilder, AnyError>>,

    /// If true, fetches and network OPs will ignore SSL errors
    pub unsafely_ignore_certificate_errors: Option<Vec<String>>,

    /// Client certificate and key for fetch
    pub client_cert_chain_and_key: TlsKeys,

    /// File fetch handler for fetch
    pub file_fetch_handler: Rc<dyn FetchHandler>,
}

pub fn extensions(options: WebOptions) -> Vec<Extension> {
    vec![
        deno_web::deno_web::init_ops_and_esm::<Permissions>(Default::default(), options.base_url),
        deno_fetch::deno_fetch::init_ops_and_esm::<Permissions>(deno_fetch::Options { ..options }),
        deno_net::deno_net::init_ops_and_esm::<Permissions>(
            options.root_cert_store_provider,
            options.unsafely_ignore_certificate_errors,
        ),
        init_web::init_ops_and_esm(),
        init_fetch::init_ops_and_esm(),
        init_net::init_ops_and_esm(),
    ]
}

pub fn snapshot_extensions(options: RuntimeOptions) -> Vec<Extension> {
    vec![
        deno_web::deno_web::init_ops::<Permissions>(Default::default(), options.base_url),
        deno_fetch::deno_fetch::init_ops::<Permissions>(deno_fetch::Options { ..options }),
        deno_net::deno_net::init_ops::<Permissions>(
            options.root_cert_store_provider,
            options.unsafely_ignore_certificate_errors,
        ),
        init_web::init_ops(),
        init_fetch::init_ops(),
        init_net::init_ops(),
    ]
}
