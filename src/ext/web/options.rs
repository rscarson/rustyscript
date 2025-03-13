use super::{DefaultWebPermissions, WebPermissions};
use deno_fetch::dns::Resolver;
use hyper_util::client::legacy::Builder;
use std::sync::Arc;

/// Options for configuring the web related extensions
#[derive(Clone)]
pub struct WebOptions {
    /// Base URL for some `deno_web` OPs
    pub base_url: Option<deno_core::ModuleSpecifier>,

    /// User agent to use for fetch
    pub user_agent: String,

    /// Root certificate store for TLS connections for fetches and network OPs
    pub root_cert_store_provider: Option<std::sync::Arc<dyn deno_tls::RootCertStoreProvider>>,

    /// Proxy for fetch
    pub proxy: Option<deno_tls::Proxy>,

    /// Request builder hook for fetch
    #[allow(clippy::type_complexity)]
    pub request_builder_hook:
        Option<fn(&mut http::Request<deno_fetch::ReqBody>) -> Result<(), deno_error::JsErrorBox>>,

    /// List of domain names or IP addresses for which fetches and network OPs will ignore SSL errors
    ///
    /// This is useful for testing with self-signed certificates
    pub unsafely_ignore_certificate_errors: Option<Vec<String>>,

    /// Client certificate and key for fetch
    pub client_cert_chain_and_key: deno_tls::TlsKeys,

    /// File fetch handler for fetch
    pub file_fetch_handler: std::rc::Rc<dyn deno_fetch::FetchHandler>,

    /// Permissions manager for sandbox-breaking extensions
    pub permissions: Arc<dyn WebPermissions>,

    /// Blob store for the web related extensions
    pub blob_store: Arc<deno_web::BlobStore>,

    ///A callback to customize HTTP client configuration.
    ///
    /// For more info on what can be configured, see [`hyper_util::client::legacy::Builder`]
    pub client_builder_hook: Option<fn(Builder) -> Builder>,

    /// Resolver for DNS resolution
    pub resolver: Resolver,

    /// OpenTelemetry configuration for the `deno_telemetry` extension
    pub telemetry_config: deno_telemetry::OtelConfig,
}

impl Default for WebOptions {
    fn default() -> Self {
        Self {
            base_url: None,
            user_agent: String::new(),
            root_cert_store_provider: None,
            proxy: None,
            request_builder_hook: None,
            unsafely_ignore_certificate_errors: None,
            client_cert_chain_and_key: deno_tls::TlsKeys::Null,
            file_fetch_handler: std::rc::Rc::new(deno_fetch::DefaultFileFetchHandler),
            permissions: Arc::new(DefaultWebPermissions),
            blob_store: Arc::new(deno_web::BlobStore::default()),
            client_builder_hook: None,
            resolver: Resolver::default(),
            telemetry_config: deno_telemetry::OtelConfig::default(),
        }
    }
}

impl WebOptions {
    /// Whitelist a domain or IP for ignoring certificate errors
    /// This is useful for testing with self-signed certificates
    pub fn whitelist_certificate_for(&mut self, domain_or_ip: impl ToString) {
        if let Some(ref mut domains) = self.unsafely_ignore_certificate_errors {
            domains.push(domain_or_ip.to_string());
        } else {
            self.unsafely_ignore_certificate_errors = Some(vec![domain_or_ip.to_string()]);
        }
    }
}
