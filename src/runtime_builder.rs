use crate::module_loader::ImportProvider;
use crate::{Error, RuntimeOptions};

/// A builder for creating a new runtime
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
    /// This can be used to add custom functionality to the runtime
    /// If the extension is for use with a snapshot, create the extension with `init_ops` instead of `init_ops_and_esm`
    #[must_use]
    pub fn with_extension(mut self, extension: deno_core::Extension) -> Self {
        self.0.extensions.push(extension);
        self
    }

    /// Add multiple extensions to the runtime
    /// This can be used to add custom functionality to the runtime
    /// If the extension is for use with a snapshot, create the extension with `init_ops` instead of `init_ops_and_esm`
    #[must_use]
    pub fn with_extensions(mut self, extensions: Vec<deno_core::Extension>) -> Self {
        self.0.extensions.extend(extensions);
        self
    }

    /// Set the default entrypoint for the runtime
    /// This is the function to use as entrypoint if a module does not provide one
    #[must_use]
    pub fn with_default_entrypoint(mut self, entrypoint: String) -> Self {
        self.0.default_entrypoint = Some(entrypoint);
        self
    }

    /// Set the timeout for the runtime
    /// This is the maximum time a script can run before it is terminated
    #[must_use]
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.0.timeout = timeout;
        self
    }

    /// Add an import provider for the module loader
    /// This can be used to load modules from custom sources
    /// Or provide custom resolution logic or caching
    #[must_use]
    pub fn with_import_provider(mut self, import_provider: Box<dyn ImportProvider>) -> Self {
        self.0.import_provider = Some(import_provider);
        self
    }

    /// Set the startup snapshot for the runtime
    /// This will reduce load times, but requires the same extensions to be loaded
    /// as when the snapshot was created
    /// If provided, user-supplied extensions must be instantiated with `init_ops` instead of `init_ops_and_esm`
    ///
    /// WARNING: Snapshots MUST be used on the same system they were created on
    #[must_use]
    pub fn with_startup_snapshot(mut self, snapshot: &'static [u8]) -> Self {
        self.0.startup_snapshot = Some(snapshot);
        self
    }

    /// Set the params used to create the underlying V8 isolate
    /// This can be used to alter the behavior of the runtime.
    /// See the `rusty_v8` documentation for more information
    #[must_use]
    pub fn with_isolate_params(mut self, params: deno_core::v8::CreateParams) -> Self {
        self.0.isolate_params = Some(params);
        self
    }

    /// Set the shared array buffer store to use for the runtime
    /// Allows data-sharing between runtimes across threads
    #[must_use]
    pub fn with_shared_array_buffer_store(
        mut self,
        store: deno_core::SharedArrayBufferStore,
    ) -> Self {
        self.0.shared_array_buffer_store = Some(store);
        self
    }

    //
    // Extension options
    //

    /// Set the options for the web extension
    #[cfg(feature = "web")]
    #[must_use]
    pub fn with_web_options(mut self, options: crate::ext::web::WebOptions) -> Self {
        self.0.extension_options.web = options;
        self
    }

    /// Set the initial seed for the crypto extension
    #[cfg(feature = "crypto")]
    #[must_use]
    pub fn with_cryto_seed(mut self, seed: u64) -> Self {
        self.0.extension_options.crypto_seed = Some(seed);
        self
    }

    /// Set the options for the io extension
    #[cfg(feature = "io")]
    #[must_use]
    pub fn with_io_pipes(mut self, pipes: deno_io::Stdio) -> Self {
        self.0.extension_options.io_pipes = Some(pipes);
        self
    }

    /// Set the options for the webstorage extension
    #[cfg(feature = "webstorage")]
    #[must_use]
    pub fn with_webstorage_origin_storage_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.0.extension_options.webstorage_origin_storage_dir = Some(dir);
        self
    }

    /// Set the options for the cache extension
    #[cfg(feature = "cache")]
    #[must_use]
    pub fn with_cache(mut self, cache: deno_cache::CreateCache<crate::CacheBackend>) -> Self {
        self.0.extension_options.cache = Some(cache);
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
    pub fn build_snapshot(self) -> Result<crate::SnapshotBuilder, Error> {
        crate::SnapshotBuilder::new(self.0)
    }
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
