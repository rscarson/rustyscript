#![allow(unused_variables)]
#![allow(clippy::derivable_impls)]
use deno_core::Extension;

pub mod rustyscript;

#[cfg(feature = "webidl")]
pub mod webidl;

#[cfg(feature = "cache")]
pub mod cache;

#[cfg(feature = "console")]
pub mod console;

#[cfg(feature = "crypto")]
pub mod crypto;

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

/// Options for configuring extensions
pub struct ExtensionOptions {
    /// Options specific to the deno_web, deno_fetch and deno_net extensions
    #[cfg(feature = "web")]
    pub web: web::WebOptions,

    /// Optional seed for the deno_crypto extension
    #[cfg(feature = "crypto")]
    pub crypto_seed: Option<u64>,

    /// Configures the stdin/out/err pipes for the deno_io extension
    #[cfg(feature = "io")]
    pub io_pipes: Option<deno_io::Stdio>,

    /// Optional path to the directory where the webstorage extension will store its data
    #[cfg(feature = "webstorage")]
    pub webstorage_origin_storage_dir: Option<std::path::PathBuf>,

    /// Optional cache configuration for the deno_cache extension
    #[cfg(feature = "cache")]
    pub cache: Option<deno_cache::CreateCache<deno_cache::SqliteBackedCache>>,

    #[cfg(feature = "websocket")]
    pub websocket: websocket::WebSocketOptions,
}

impl Default for ExtensionOptions {
    fn default() -> Self {
        Self {
            #[cfg(feature = "web")]
            web: web::WebOptions::default(),

            #[cfg(feature = "crypto")]
            crypto_seed: None,

            #[cfg(feature = "io")]
            io_pipes: Some(Default::default()),

            #[cfg(feature = "webstorage")]
            webstorage_origin_storage_dir: None,

            #[cfg(feature = "cache")]
            cache: None,

            #[cfg(feature = "websocket")]
            websocket: websocket::WebSocketOptions::from(web::WebOptions::default())
        }
    }
}

///
/// Add up all required extensions
pub fn all_extensions(
    user_extensions: Vec<Extension>,
    options: ExtensionOptions,
) -> Vec<Extension> {
    let mut extensions = rustyscript::extensions();

    #[cfg(feature = "webidl")]
    extensions.extend(webidl::extensions());

    #[cfg(feature = "console")]
    extensions.extend(console::extensions());

    #[cfg(feature = "url")]
    extensions.extend(url::extensions());

    #[cfg(feature = "web")]
    extensions.extend(web::extensions(options.web.clone()));

    #[cfg(feature = "cache")]
    extensions.extend(cache::extensions(options.cache));

    #[cfg(all(not(feature = "web"), feature = "web_stub"))]
    extensions.extend(web_stub::extensions());

    #[cfg(feature = "crypto")]
    extensions.extend(crypto::extensions(options.crypto_seed));

    #[cfg(feature = "io")]
    extensions.extend(io::extensions(options.io_pipes));

    #[cfg(feature = "webstorage")]
    extensions.extend(webstorage::extensions(
        options.webstorage_origin_storage_dir,
    ));

    #[cfg(feature = "websocket")]
    extensions.extend(websocket::extensions(websocket::WebSocketOptions::from(options.web)));

    extensions.extend(user_extensions);
    extensions
}

///
/// Add up all required extensions, in snapshot mode
pub fn all_snapshot_extensions(
    user_extensions: Vec<Extension>,
    options: ExtensionOptions,
) -> Vec<Extension> {
    let mut extensions = rustyscript::snapshot_extensions();

    #[cfg(feature = "webidl")]
    extensions.extend(webidl::snapshot_extensions());

    #[cfg(feature = "console")]
    extensions.extend(console::snapshot_extensions());

    #[cfg(feature = "url")]
    extensions.extend(url::snapshot_extensions());

    #[cfg(feature = "web")]
    extensions.extend(web::snapshot_extensions(options.web.clone()));

    #[cfg(feature = "cache")]
    extensions.extend(cache::snapshot_extensions(options.cache));

    #[cfg(all(not(feature = "web"), feature = "web_stub"))]
    extensions.extend(web_stub::snapshot_extensions());

    #[cfg(feature = "crypto")]
    extensions.extend(crypto::snapshot_extensions(options.crypto_seed));

    #[cfg(feature = "io")]
    extensions.extend(io::snapshot_extensions(options.io_pipes));

    #[cfg(feature = "webstorage")]
    extensions.extend(webstorage::snapshot_extensions(
        options.webstorage_origin_storage_dir,
    ));

    #[cfg(feature = "websocket")]
    extensions.extend(websocket::snapshot_extensions(websocket::WebSocketOptions::from(options.web)));

    extensions.extend(user_extensions);
    extensions
}
