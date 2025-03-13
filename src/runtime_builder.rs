use crate::module_loader::ImportProvider;
use crate::{Error, RuntimeOptions};

/// A builder for creating a new runtime
///
/// Just a helper wrapper around `RuntimeOptions` for `Runtime` and `SnapshotBuilder`
///
/// # Example
/// ```rust
/// use rustyscript::RuntimeBuilder;
///
/// let runtime = RuntimeBuilder::new()
///     .with_timeout(std::time::Duration::from_secs(5))
///     .with_default_entrypoint("main".to_string())
///     .with_cryto_seed(42)
///     .build()
///     .expect("Failed to create runtime");
/// ```
pub struct RuntimeBuilder(RuntimeOptions);
impl RuntimeBuilder {
    /// Create a new runtime builder with default options
    #[must_use]
    pub fn new() -> Self {
        Self(RuntimeOptions::default())
    }

    /// Add an extension to the runtime
    ///
    /// This can be used to add custom functionality to the runtime
    ///
    /// If the extension is for use with a snapshot, create the extension with `init_ops` instead of `init_ops_and_esm`
    #[must_use]
    pub fn with_extension(mut self, extension: deno_core::Extension) -> Self {
        self.0.extensions.push(extension);
        self
    }

    /// Add multiple extensions to the runtime
    ///
    /// This can be used to add custom functionality to the runtime
    ///
    /// If the extension is for use with a snapshot, create the extension with `init_ops` instead of `init_ops_and_esm`
    #[must_use]
    pub fn with_extensions(mut self, extensions: Vec<deno_core::Extension>) -> Self {
        self.0.extensions.extend(extensions);
        self
    }

    /// Set the default entrypoint for the runtime
    ///
    /// This is the function to use as entrypoint if a module does not provide one
    #[must_use]
    pub fn with_default_entrypoint(mut self, entrypoint: String) -> Self {
        self.0.default_entrypoint = Some(entrypoint);
        self
    }

    /// Set the timeout for the runtime
    ///
    /// This is the maximum time a script can run before it is terminated
    #[must_use]
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.0.timeout = timeout;
        self
    }

    /// Optional maximum heap size for the runtime
    #[must_use]
    pub fn with_max_heap_size(mut self, max_heap_size: usize) -> Self {
        self.0.max_heap_size = Some(max_heap_size);
        self
    }

    /// Optional import provider for the module loader
    #[must_use]
    pub fn with_import_provider(mut self, import_provider: Box<dyn ImportProvider>) -> Self {
        self.0.import_provider = Some(import_provider);
        self
    }

    /// Set the startup snapshot for the runtime
    ///
    /// This will reduce load times, but requires the same extensions to be loaded as when the snapshot was created
    ///
    /// If provided, user-supplied extensions must be instantiated with `init_ops` instead of `init_ops_and_esm`
    ///
    /// WARNING: Snapshots MUST be used on the same system they were created on
    #[must_use]
    pub fn with_startup_snapshot(mut self, snapshot: &'static [u8]) -> Self {
        self.0.startup_snapshot = Some(snapshot);
        self
    }

    /// Set the params used to create the underlying V8 isolate
    ///
    /// This can be used to alter the behavior of the runtime.
    ///
    /// See the `rusty_v8` documentation for more information
    #[must_use]
    pub fn with_isolate_params(mut self, params: deno_core::v8::CreateParams) -> Self {
        self.0.isolate_params = Some(params);
        self
    }

    /// Set the shared array buffer store to use for the runtime
    ///
    /// Allows data-sharing between runtimes across threads
    #[must_use]
    pub fn with_shared_array_buffer_store(
        mut self,
        store: deno_core::SharedArrayBufferStore,
    ) -> Self {
        self.0.shared_array_buffer_store = Some(store);
        self
    }

    /// Add to a whitelist of custom schema prefixes that are allowed to be loaded from javascript
    ///
    /// By default only http/https (`url_import` crate feature), and file (`fs_import` crate feature) are allowed
    #[must_use]
    pub fn with_schema(mut self, schema: impl ToString) -> Self {
        self.0.schema_whlist.insert(schema.to_string());
        self
    }

    //
    // Extension options
    //

    /// Set the initial seed for the crypto extension
    #[cfg(feature = "crypto")]
    #[cfg_attr(docsrs, doc(cfg(feature = "crypto")))]
    #[must_use]
    pub fn with_cryto_seed(mut self, seed: u64) -> Self {
        self.0.extension_options.crypto_seed = Some(seed);
        self
    }

    /// Set the options for the io extension
    #[cfg(feature = "io")]
    #[cfg_attr(docsrs, doc(cfg(feature = "io")))]
    #[must_use]
    pub fn with_io_pipes(mut self, pipes: deno_io::Stdio) -> Self {
        self.0.extension_options.io_pipes = Some(pipes);
        self
    }

    /// Set the options for the webstorage extension
    #[cfg(feature = "webstorage")]
    #[cfg_attr(docsrs, doc(cfg(feature = "webstorage")))]
    #[must_use]
    pub fn with_webstorage_origin_storage_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.0.extension_options.webstorage_origin_storage_dir = Some(dir);
        self
    }

    /// Set the options for the cache extension
    #[cfg(feature = "cache")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cache")))]
    #[must_use]
    pub fn with_cache(mut self, cache: deno_cache::CreateCache) -> Self {
        self.0.extension_options.cache = Some(cache);
        self
    }

    /// Set the options for the broadcast channel extension
    #[cfg(feature = "broadcast_channel")]
    #[cfg_attr(docsrs, doc(cfg(feature = "broadcast_channel")))]
    #[must_use]
    pub fn with_broadcast_channel(
        mut self,
        channel: deno_broadcast_channel::InMemoryBroadcastChannel,
    ) -> Self {
        self.0.extension_options.broadcast_channel = channel;
        self
    }

    /// Set the options for the kv store extension
    #[cfg(feature = "kv")]
    #[cfg_attr(docsrs, doc(cfg(feature = "kv")))]
    #[must_use]
    pub fn with_kv_store(mut self, kv_store: crate::KvStore) -> Self {
        self.0.extension_options.kv_store = kv_store;
        self
    }

    /// Set the options for the node extension
    #[cfg(feature = "node_experimental")]
    #[cfg_attr(docsrs, doc(cfg(feature = "node_experimental")))]
    #[must_use]
    pub fn with_node_resolver(mut self, resolver: std::sync::Arc<crate::RustyResolver>) -> Self {
        self.0.extension_options.node_resolver = resolver;
        self
    }

    //
    // Web options
    //

    /// Base URL for some `deno_web` OPs
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    #[must_use]
    pub fn with_web_base_url(mut self, base_url: deno_core::ModuleSpecifier) -> Self {
        self.0.extension_options.web.base_url = Some(base_url);
        self
    }

    /// User agent to use for fetch
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    #[must_use]
    pub fn with_web_user_agent(mut self, user_agent: String) -> Self {
        self.0.extension_options.web.user_agent = user_agent;
        self
    }

    /// Root certificate store for TLS connections for fetches and network OPs
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    #[must_use]
    pub fn with_web_root_cert_store_provider(
        mut self,
        root_cert_store_provider: std::sync::Arc<dyn deno_tls::RootCertStoreProvider>,
    ) -> Self {
        self.0.extension_options.web.root_cert_store_provider = Some(root_cert_store_provider);
        self
    }

    /// Proxy for fetch
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    #[must_use]
    pub fn with_web_proxy(mut self, proxy: deno_tls::Proxy) -> Self {
        self.0.extension_options.web.proxy = Some(proxy);
        self
    }

    /// Request builder hook for fetch
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    #[must_use]
    pub fn with_web_request_builder_hook(
        mut self,
        hook: fn(&mut http::Request<deno_fetch::ReqBody>) -> Result<(), deno_error::JsErrorBox>,
    ) -> Self {
        self.0.extension_options.web.request_builder_hook = Some(hook);
        self
    }

    /// List of domain names or IP addresses for which fetches and network OPs will ignore SSL errors
    ///
    /// This is useful for testing with self-signed certificates
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    #[must_use]
    pub fn with_web_unsafely_ignored_certificate_errors(mut self, domain: impl ToString) -> Self {
        match &mut self
            .0
            .extension_options
            .web
            .unsafely_ignore_certificate_errors
        {
            Some(vec) => vec.push(domain.to_string()),
            None => {
                self.0
                    .extension_options
                    .web
                    .unsafely_ignore_certificate_errors = Some(vec![domain.to_string()]);
            }
        }

        self
    }

    /// Client certificate and key for fetch
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    #[must_use]
    pub fn with_web_client_cert_chain_and_key(mut self, keys: deno_tls::TlsKeys) -> Self {
        self.0.extension_options.web.client_cert_chain_and_key = keys;
        self
    }

    /// File fetch handler for fetch
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    #[must_use]
    pub fn with_web_file_fetch_handler(
        mut self,
        handler: std::rc::Rc<dyn deno_fetch::FetchHandler>,
    ) -> Self {
        self.0.extension_options.web.file_fetch_handler = handler;
        self
    }

    /// Permissions manager for sandbox-breaking extensions
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    #[must_use]
    pub fn with_web_permissions(
        mut self,
        permissions: std::sync::Arc<dyn crate::ext::web::WebPermissions>,
    ) -> Self {
        self.0.extension_options.web.permissions = permissions;
        self
    }

    /// Blob store for the web related extensions
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    #[must_use]
    pub fn with_web_blob_store(mut self, blob_store: std::sync::Arc<deno_web::BlobStore>) -> Self {
        self.0.extension_options.web.blob_store = blob_store;
        self
    }

    /// A callback to customize HTTP client configuration.
    ///
    /// For more info on what can be configured, see [`hyper_util::client::legacy::Builder`]
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    #[must_use]
    pub fn with_web_client_builder_hook(
        mut self,
        hook: Option<
            fn(hyper_util::client::legacy::Builder) -> hyper_util::client::legacy::Builder,
        >,
    ) -> Self {
        self.0.extension_options.web.client_builder_hook = hook;
        self
    }

    /// Resolver for DNS resolution
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    #[must_use]
    pub fn with_web_resolver(mut self, resolver: deno_fetch::dns::Resolver) -> Self {
        self.0.extension_options.web.resolver = resolver;
        self
    }

    /// Consume the builder and create a new runtime with the given options
    ///
    /// # Errors
    /// Will return an error if the runtime cannot be created (usually an issue with extensions)
    pub fn build(self) -> Result<crate::Runtime, Error> {
        crate::Runtime::new(self.0)
    }

    /// Consume the builder and create a new snapshot runtime with the given options
    ///
    /// # Errors
    /// Will return an error if the runtime cannot be created (usually an issue with extensions)
    #[cfg(feature = "snapshot_builder")]
    #[cfg_attr(docsrs, doc(cfg(feature = "snapshot_builder")))]
    pub fn build_snapshot(self) -> Result<crate::SnapshotBuilder, Error> {
        crate::SnapshotBuilder::new(self.0)
    }
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
