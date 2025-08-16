#![allow(unused_variables)]
#![allow(clippy::derivable_impls)]

use deno_core::{
    v8::{BackingStore, SharedRef},
    CrossIsolateStore, Extension,
};

pub mod rustyscript;

trait ExtensionTrait<A> {
    fn init(options: A) -> Extension;

    // Clears the js and esm files for warmup to avoid reloading them
    fn for_warmup(mut ext: Extension) -> Extension {
        ext.js_files = ::std::borrow::Cow::Borrowed(&[]);
        ext.esm_files = ::std::borrow::Cow::Borrowed(&[]);
        ext.esm_entry_point = ::std::option::Option::None;

        ext
    }

    /// Builds an extension
    fn build(options: A, is_snapshot: bool) -> Extension {
        let ext: Extension = Self::init(options);
        if is_snapshot {
            Self::for_warmup(ext)
        } else {
            ext
        }
    }
}

#[cfg(feature = "webidl")]
pub mod webidl;

#[cfg(feature = "broadcast_channel")]
pub mod broadcast_channel;

#[cfg(feature = "cache")]
pub mod cache;

#[cfg(feature = "console")]
pub mod console;

#[cfg(feature = "crypto")]
pub mod crypto;

#[cfg(feature = "fs")]
pub mod fs;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "url")]
pub mod url;

#[cfg(feature = "web")]
pub mod web;

#[cfg(all(not(feature = "web"), feature = "web_stub"))]
pub mod web_stub;

#[cfg(feature = "io")]
pub mod io;

#[cfg(feature = "webstorage")]
pub mod webstorage;

#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(feature = "webgpu")]
pub mod webgpu;

#[cfg(feature = "kv")]
pub mod kv;

#[cfg(feature = "cron")]
pub mod cron;

#[cfg(feature = "node_experimental")]
pub mod napi;
#[cfg(feature = "node_experimental")]
pub mod node;
#[cfg(feature = "node_experimental")]
pub mod runtime;

/// Options for configuring extensions
pub struct ExtensionOptions {
    /// Options specific to the `deno_web`, `deno_fetch` and `deno_net` extensions
    ///
    /// Requires the `web` feature to be enabled
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    pub web: web::WebOptions,

    /// Optional seed for the `deno_crypto` extension
    ///
    /// Requires the `crypto` feature to be enabled
    #[cfg(feature = "crypto")]
    #[cfg_attr(docsrs, doc(cfg(feature = "crypto")))]
    pub crypto_seed: Option<u64>,

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
    pub kv_store: kv::KvStore,

    /// Package resolver for the `deno_node` extension
    /// `RustyResolver` allows you to select the base dir for modules
    /// as well as the filesystem implementation to use
    ///
    /// Requires the `node_experimental` feature to be enabled
    #[cfg(feature = "node_experimental")]
    #[cfg_attr(docsrs, doc(cfg(feature = "node_experimental")))]
    pub node_resolver: std::sync::Arc<node::resolvers::RustyResolver>,
}

impl Default for ExtensionOptions {
    fn default() -> Self {
        Self {
            #[cfg(feature = "web")]
            web: web::WebOptions::default(),

            #[cfg(feature = "crypto")]
            crypto_seed: None,

            #[cfg(feature = "io")]
            io_pipes: Some(deno_io::Stdio::default()),

            #[cfg(feature = "webstorage")]
            webstorage_origin_storage_dir: None,

            #[cfg(feature = "cache")]
            cache: None,

            #[cfg(feature = "fs")]
            filesystem: std::sync::Arc::new(deno_fs::RealFs),

            #[cfg(feature = "broadcast_channel")]
            broadcast_channel: deno_broadcast_channel::InMemoryBroadcastChannel::default(),

            #[cfg(feature = "kv")]
            kv_store: kv::KvStore::default(),

            #[cfg(feature = "node_experimental")]
            node_resolver: std::sync::Arc::new(node::resolvers::RustyResolver::default()),
        }
    }
}

pub(crate) fn all_extensions(
    user_extensions: Vec<Extension>,
    options: ExtensionOptions,
    shared_array_buffer_store: Option<CrossIsolateStore<SharedRef<BackingStore>>>,
    is_snapshot: bool,
) -> Vec<Extension> {
    let mut extensions = rustyscript::extensions(is_snapshot);

    #[cfg(feature = "webidl")]
    extensions.extend(webidl::extensions(is_snapshot));

    #[cfg(feature = "console")]
    extensions.extend(console::extensions(is_snapshot));

    #[cfg(feature = "url")]
    extensions.extend(url::extensions(is_snapshot));

    #[cfg(feature = "web")]
    extensions.extend(web::extensions(options.web.clone(), is_snapshot));

    #[cfg(feature = "broadcast_channel")]
    extensions.extend(broadcast_channel::extensions(
        options.broadcast_channel.clone(),
        is_snapshot,
    ));

    #[cfg(feature = "cache")]
    extensions.extend(cache::extensions(options.cache.clone(), is_snapshot));

    #[cfg(all(not(feature = "web"), feature = "web_stub"))]
    extensions.extend(web_stub::extensions(is_snapshot));

    #[cfg(feature = "crypto")]
    extensions.extend(crypto::extensions(options.crypto_seed, is_snapshot));

    #[cfg(feature = "io")]
    extensions.extend(io::extensions(options.io_pipes.clone(), is_snapshot));

    #[cfg(feature = "webstorage")]
    extensions.extend(webstorage::extensions(
        options.webstorage_origin_storage_dir.clone(),
        is_snapshot,
    ));

    #[cfg(feature = "websocket")]
    extensions.extend(websocket::extensions(options.web.clone(), is_snapshot));

    #[cfg(feature = "fs")]
    extensions.extend(fs::extensions(options.filesystem.clone(), is_snapshot));

    #[cfg(feature = "http")]
    extensions.extend(http::extensions((), is_snapshot));

    #[cfg(feature = "ffi")]
    extensions.extend(ffi::extensions(is_snapshot));

    #[cfg(feature = "kv")]
    extensions.extend(kv::extensions(options.kv_store.clone(), is_snapshot));

    #[cfg(feature = "webgpu")]
    extensions.extend(webgpu::extensions(is_snapshot));

    #[cfg(feature = "cron")]
    extensions.extend(cron::extensions(is_snapshot));

    #[cfg(feature = "node_experimental")]
    {
        extensions.extend(napi::extensions(is_snapshot));
        extensions.extend(node::extensions(options.node_resolver.clone(), is_snapshot));

        extensions.extend(runtime::extensions(
            &options,
            shared_array_buffer_store,
            is_snapshot,
        ));
    }

    extensions.extend(user_extensions);
    extensions
}
