use std::collections::HashMap;

use deno_core::Extension;

use crate::ExtensionOptions;

/// An ordered set of extensions to load into a runtime
///
/// [`ExtensionList::new_default`] Can be used to create a set of extensions defined
/// by the active crate features
///
/// [`ExtensionList::new_sandboxed`] Can be used to create a set of extensions which preserves the sandboxing guarantees
///
/// [`ExtensionList::append`] Can then add additional extensions to the list
///
/// [`ExtensionList::unsafe_remove`] Can be used to remove extensions from the load order for a runtime
/// but care must be taken; it will not check for dependencies
///
/// The dependency tree described in `Cargo.toml` MUST be preserved
///
/// In addition, any associated loaders like `init_console` also need to be removed
///
/// Violating the dependency tree can cause panics, runtime crashes, or unpredictable behavior
pub struct ExtensionList {
    inner: Vec<Extension>,
    options: ExtensionOptions,
}
impl ExtensionList {
    /// Create an empty extension list
    ///
    /// This is not exposed, because it will not provide -any- runtime initialization!
    /// - Registered rust functions would not be available from JS
    /// - Entrypoints would not be registered
    ///
    /// Use [`ExtensionList::new_default`] to build a list defined by available crate features
    #[must_use]
    pub fn new_empty(options: ExtensionOptions) -> Self {
        Self {
            inner: Vec::new(),
            options,
        }
    }

    /// Access the extension options, mutably
    ///
    /// This is not exposed since it cannot be used usefully without `new_empty`, which is also not exposed
    pub(crate) fn options_mut(&mut self) -> &mut ExtensionOptions {
        &mut self.options
    }

    /// Add an extension to the end of the load order
    #[must_use]
    pub fn with_appended(mut self, ext: Extension) -> Self {
        self.append(ext);
        self
    }

    /// Add an extension to the end of the load order
    pub fn append(&mut self, ext: Extension) {
        self.inner.push(ext);
    }

    /// Add multiple extensions to the end of the load order
    pub fn extend(&mut self, exts: impl IntoIterator<Item = Extension>) {
        self.inner.extend(exts);
    }

    /// If no extensions depend on the one named, remove it. Otherwise, lists direct dependencies
    ///
    /// # Errors
    /// - If other extensions depend on the one named, returns their names
    /// - If the named extension is not found, return the name
    pub fn try_unload<'name>(&mut self, name: &'name str) -> Result<Extension, Vec<&'name str>> {
        let rdeps = get_reverse_dependencies(name);
        if rdeps.is_empty() {
            // No reverse dependencies, safe to unload
            let Some(ext) = (unsafe { self.unsafely_remove(name) }) else {
                return Err(vec![name]);
            };
            Ok(ext)
        } else {
            // List reverse dependencies
            Err(rdeps.to_vec())
        }
    }

    /// Remove the extension with the given name, and everything that depends on it (recursively)
    pub fn unload(&mut self, name: &str) -> Vec<Extension> {
        /* Safety: We use the dep tree to ensure we are removing all dependencies */
        let Some(ext) = (unsafe { self.unsafely_remove(name) }) else {
            return Vec::new();
        };

        let mut removed = vec![ext];
        for rdep in get_reverse_dependencies(name) {
            removed.extend(self.unload(rdep));
        }

        removed
    }

    /// Remove an extension by name, like `deno_console`
    ///
    /// A safer alternative is [`ExtensionsList::unload`]
    ///
    /// # Safety
    ///
    /// The dependency tree described in `Cargo.toml` MUST be preserved
    ///
    /// In addition, any associated loaders like `init_console` also need to be removed
    ///
    /// Violating the dependency tree can cause panics, runtime crashes, or unpredictable behavior
    #[must_use]
    pub unsafe fn unsafely_remove(&mut self, name: &str) -> Option<Extension> {
        if let Some(pos) = self.inner.iter().position(|e| e.name == name) {
            Some(self.inner.remove(pos))
        } else {
            None
        }
    }

    /// Clear ESM files from all extensions to prevent reload with snapshot warmup
    pub(crate) fn clear_esm(&mut self) {
        for ext in &mut self.inner {
            ext.js_files = ::std::borrow::Cow::Borrowed(&[]);
            ext.esm_files = ::std::borrow::Cow::Borrowed(&[]);
            ext.esm_entry_point = ::std::option::Option::None;
        }
    }

    /// List all extensions in the order they will be loaded
    #[must_use]
    pub fn as_slice(&self) -> &[Extension] {
        &self.inner
    }

    /// Get an iterator over the extensions
    pub fn iter(&self) -> std::slice::Iter<'_, Extension> {
        self.inner.iter()
    }

    /// Get the options for the extension list
    #[must_use]
    pub fn options(&self) -> &ExtensionOptions {
        &self.options
    }

    /// Convert the extension list into its inner components
    #[must_use]
    pub fn into_inner(self) -> (Vec<Extension>, ExtensionOptions) {
        (self.inner, self.options)
    }
}
impl<'a> IntoIterator for &'a ExtensionList {
    type Item = &'a Extension;
    type IntoIter = std::slice::Iter<'a, Extension>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Manually maintained dependency tree for all feature extensions
/// 
/// Curated from Cargo.toml and the extension load functions
#[rustfmt::skip]
const DEPENDENCY_TREE: &[(&str, &[&str])] = &[
    ("broadcast_channel", &["rustyscript", "deno_broadcast_channel"]),
    ("deno_broadcast_channel", &["web", "webidl"]),

    ("cache", &["rustyscript", "deno_cache"]),
    ("deno_cache", &["web", "webidl"]),

    ("console", &["rustyscript", "deno_console"]),
    ("deno_console", &[]),

    ("cron", &["rustyscript", "deno_cron"]),
    ("deno_cron", &["console"]),

    ("crypto", &["rustyscript", "deno_crypto"]),
    ("deno_crypto", &["webidl"]),

    ("ffi", &["rustyscript", "deno_crypto"]),
    ("deno_ffi", &["web"]),

    ("fs", &["rustyscript", "deno_fs"]),
    ("deno_fs", &["web", "io"]),

    ("http", &["rustyscript", "deno_http"]),
    ("deno_http", &["web", "websocket"]),

    ("kv", &["rustyscript", "deno_kv"]),
    ("deno_kv", &["web", "console"]),

    ("io", &["deno_io", "deno_tty"]),
    ("deno_io", &["web"]),
    ("deno_tty", &["web"]),

    ("url", &["rustyscript", "deno_url"]),
    ("deno_url", &["webidl"]),

    ("web", &["rustyscript", "fetch", "net", "telemetry", "deno_web"]),
    ("fetch", &["rustyscript", "web", "net", "telemetry", "deno_fetch"]),
    ("net", &["rustyscript", "web", "fetch", "telemetry", "deno_net"]),
    ("telemetry", &["rustyscript", "web", "fetch", "net", "deno_telemetry"]),
    ("deno_web", &["webidl", "fs", "console", "url", "crypto"]),
    ("deno_fetch", &["webidl", "fs", "console", "url", "crypto"]),
    ("deno_net", &["webidl", "fs", "console", "url", "crypto"]),
    ("deno_telemetry", &["webidl", "fs", "console", "url", "crypto"]),

    ("webgpu", &["rustyscript", "deno_webgpu"]),
    ("deno_webgpu", &["web"]),

    ("webidl", &["rustyscript", "deno_webidl"]),
    ("deno_webidl", &[]),

    ("webstorage", &["rustyscript", "deno_webstorage"]),
    ("deno_webstorage", &["webidl"]),

    ("websocket", &["rustyscript", "deno_websocket"]),
    ("deno_websocket", &["web"]),

    ("node", &["rustyscript", "napi", "runtime", "deno_node"]),
    ("napi", &["rustyscript", "node", "runtime", "deno_napi"]),
    ("runtime", &["rustyscript", "node", "node", "deno_runtime"]),
    ("deno_node", &[
        "web", "webstorage", "websocket", "http", "url", "crypto", "console", "broadcast_channel",
        "fs", "io", "cache", "ffi", "webgpu", "kv", "cron"
    ]),
    ("deno_napi", &[
        "web", "webstorage", "websocket", "http", "url", "crypto", "console", "broadcast_channel",
        "fs", "io", "cache", "ffi", "webgpu", "kv", "cron"
    ]),
    ("deno_runtime", &[
        "deno_fs_events", "deno_bootstrap", "deno_os", "deno_process", "deno_web_worker", "deno_worker_host", "deno_permissions",
        "web", "webstorage", "websocket", "http", "url", "crypto", "console", "broadcast_channel",
        "fs", "io", "cache", "ffi", "webgpu", "kv", "cron"
    ]),
];

/// For a given extension `name`, get all dependencies which directly depend on it
fn get_reverse_dependencies(name: &str) -> &'static [&'static str] {
    static REVERSE_DEPENDENCIES: std::sync::OnceLock<HashMap<&str, Vec<&str>>> =
        std::sync::OnceLock::new();

    //
    // Load the reverse dependency map only on first access
    let map = REVERSE_DEPENDENCIES.get_or_init(|| {
        let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
        for &(ext, deps) in DEPENDENCY_TREE {
            for &dep in deps {
                map.entry(dep).or_default().push(ext);
            }
        }
        map
    });

    map.get(name).map_or(&[], |v| &v[..])
}

macro_rules! mod_if {
    ($name:ident, cfg = $cfg:meta) => {
        #[cfg($cfg)]
        pub mod $name;
    };
    ($name:ident, feature = $feature:literal) => {
        #[cfg(feature = $feature)]
        pub mod $name;
    };
}
