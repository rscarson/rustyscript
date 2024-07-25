#![allow(unused_variables)]
#![allow(clippy::derivable_impls)]
use deno_core::Extension;

pub mod rustyscript;

trait ExtensionTrait<A> {
    fn init(options: A) -> Extension;

    /// Makes a call to `init_ops_and_esm` equivalent to `init_ops`
    fn set_esm(mut ext: Extension, is_snapshot: bool) -> Extension {
        if is_snapshot {
            ext.js_files = ::std::borrow::Cow::Borrowed(&[]);
            ext.esm_files = ::std::borrow::Cow::Borrowed(&[]);
            ext.esm_entry_point = ::std::option::Option::None;
        }
        ext
    }

    /// Builds an extension
    fn build(options: A, is_snapshot: bool) -> Extension {
        let ext = Self::init(options);
        Self::set_esm(ext, is_snapshot)
    }
}

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
    /// Options specific to the `deno_web`, `deno_fetch` and `deno_net` extensions
    #[cfg(feature = "web")]
    pub web: web::WebOptions,

    /// Optional seed for the `deno_crypto` extension
    #[cfg(feature = "crypto")]
    pub crypto_seed: Option<u64>,

    /// Configures the stdin/out/err pipes for the `deno_io` extension
    #[cfg(feature = "io")]
    pub io_pipes: Option<deno_io::Stdio>,

    /// Optional path to the directory where the webstorage extension will store its data
    #[cfg(feature = "webstorage")]
    pub webstorage_origin_storage_dir: Option<std::path::PathBuf>,

    /// Optional cache configuration for the `deno_cache` extension
    #[cfg(feature = "cache")]
    pub cache: Option<deno_cache::CreateCache<deno_cache::SqliteBackedCache>>,
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
        }
    }
}

pub(crate) fn all_extensions(
    user_extensions: Vec<Extension>,
    options: ExtensionOptions,
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

    #[cfg(feature = "cache")]
    extensions.extend(cache::extensions(options.cache, is_snapshot));

    #[cfg(all(not(feature = "web"), feature = "web_stub"))]
    extensions.extend(web_stub::extensions(is_snapshot));

    #[cfg(feature = "crypto")]
    extensions.extend(crypto::extensions(options.crypto_seed, is_snapshot));

    #[cfg(feature = "io")]
    extensions.extend(io::extensions(options.io_pipes, is_snapshot));

    #[cfg(feature = "webstorage")]
    extensions.extend(webstorage::extensions(
        options.webstorage_origin_storage_dir,
        is_snapshot,
    ));

    #[cfg(feature = "websocket")]
    extensions.extend(websocket::extensions(options.web.clone(), is_snapshot));

    extensions.extend(user_extensions);
    extensions
}
