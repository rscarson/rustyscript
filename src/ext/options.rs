use deno_core::{
    v8::{BackingStore, SharedRef},
    CrossIsolateStore,
};

/// Options for configuring extensions
pub struct ExtensionOptions {
    /// Options specific to the `deno_web`, `deno_fetch` and `deno_net` extensions
    ///
    /// Requires the `web` feature to be enabled
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    pub web: super::web::WebOptions,

    /// Optional seed for the `deno_crypto` extension
    ///
    /// Requires the `crypto` feature to be enabled
    #[cfg(feature = "crypto")]
    #[cfg_attr(docsrs, doc(cfg(feature = "crypto")))]
    pub crypto_seed: Option<u64>,

    /// Optional loader for FFI addons
    ///
    /// Requires the `ffi` feature to be enabled
    #[cfg(feature = "ffi")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ffi")))]
    pub ffi_addon_loader: Option<deno_ffi::DenoRtNativeAddonLoaderRc>,

    /// Configures the stdin/out/err pipes for the `deno_io` extension
    ///
    /// Requires the `io` feature to be enabled
    #[cfg(feature = "io")]
    #[cfg_attr(docsrs, doc(cfg(feature = "io")))]
    pub io_pipes: Option<deno_io::Stdio>,

    /// Optional path to the directory where the webstorage extension will store its data
    ///
    /// Requires the `webstorage` feature to be enabled
    #[cfg(feature = "webstorage")]
    #[cfg_attr(docsrs, doc(cfg(feature = "webstorage")))]
    pub webstorage_origin_storage_dir: Option<std::path::PathBuf>,

    /// Optional cache configuration for the `deno_cache` extension
    ///
    /// Requires the `cache` feature to be enabled
    #[cfg(feature = "cache")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cache")))]
    pub cache: Option<deno_cache::CreateCache>,

    /// Filesystem implementation for the `deno_fs` extension
    ///
    /// Requires the `fs` feature to be enabled
    #[cfg(feature = "fs")]
    #[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
    pub filesystem: deno_fs::FileSystemRc,

    /// Shared in-memory broadcast channel for the `deno_broadcast_channel` extension
    /// Also used by `WebWorker` to communicate with the main thread, if node is enabled
    ///
    /// Requires the `broadcast_channel` feature to be enabled
    #[cfg(feature = "broadcast_channel")]
    #[cfg_attr(docsrs, doc(cfg(feature = "broadcast_channel")))]
    pub broadcast_channel: deno_broadcast_channel::InMemoryBroadcastChannel,

    /// Key-value store for the `deno_kv` extension
    ///
    /// Requires the `kv` feature to be enabled
    #[cfg(feature = "kv")]
    #[cfg_attr(docsrs, doc(cfg(feature = "kv")))]
    pub kv_store: super::kv::KvStore,

    /// Package resolver for the `deno_node` extension
    /// `RustyResolver` allows you to select the base dir for modules
    /// as well as the filesystem implementation to use
    ///
    /// Requires the `node_experimental` feature to be enabled
    #[cfg(feature = "node_experimental")]
    #[cfg_attr(docsrs, doc(cfg(feature = "node_experimental")))]
    pub node_resolver: std::sync::Arc<super::node::resolvers::RustyResolver>,

    /// Optional shared array buffer store to use for the runtime.
    ///
    /// Allows data-sharing between runtimes across threads
    pub shared_array_buffer_store: Option<CrossIsolateStore<SharedRef<BackingStore>>>,
}

impl Default for ExtensionOptions {
    fn default() -> Self {
        Self {
            #[cfg(feature = "web")]
            web: super::web::WebOptions::default(),

            #[cfg(feature = "crypto")]
            crypto_seed: None,

            #[cfg(feature = "io")]
            io_pipes: Some(deno_io::Stdio::default()),

            #[cfg(feature = "ffi")]
            ffi_addon_loader: None,

            #[cfg(feature = "webstorage")]
            webstorage_origin_storage_dir: None,

            #[cfg(feature = "cache")]
            cache: None,

            #[cfg(feature = "fs")]
            filesystem: std::sync::Arc::new(deno_fs::RealFs),

            #[cfg(feature = "broadcast_channel")]
            broadcast_channel: deno_broadcast_channel::InMemoryBroadcastChannel::default(),

            #[cfg(feature = "kv")]
            kv_store: super::kv::KvStore::default(),

            #[cfg(feature = "node_experimental")]
            node_resolver: std::sync::Arc::new(super::node::resolvers::RustyResolver::default()),

            shared_array_buffer_store: None,
        }
    }
}
