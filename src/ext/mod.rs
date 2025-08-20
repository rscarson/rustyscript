#![allow(unused_variables)]
#![allow(clippy::derivable_impls)]

mod options;
pub use options::ExtensionOptions;

#[macro_use]
mod extensionlist;
pub use extensionlist::ExtensionList;

//
// Base runtime extension
pub mod rustyscript;

//
// Safe extensions
mod_if!(webidl, feature = "webidl");
mod_if!(console, feature = "console");
mod_if!(url, feature = "url");
mod_if!(crypto, feature = "crypto");
mod_if!(
    web_stub,
    cfg = all(not(feature = "web"), feature = "web_stub")
);

//
// IO extensions
mod_if!(fs, feature = "fs");
mod_if!(io, feature = "io");
mod_if!(cache, feature = "cache");
mod_if!(ffi, feature = "ffi");
mod_if!(webgpu, feature = "webgpu");
mod_if!(kv, feature = "kv");
mod_if!(cron, feature = "cron");

//
// Networking extensions
mod_if!(broadcast_channel, feature = "broadcast_channel");
mod_if!(http, feature = "http");
mod_if!(web, feature = "web");
mod_if!(webstorage, feature = "webstorage");
mod_if!(websocket, feature = "websocket");

//
// Node.js experimental features
mod_if!(napi, feature = "node_experimental");
mod_if!(node, feature = "node_experimental");
mod_if!(runtime, feature = "node_experimental");

#[allow(unused_mut)] // Some feature combinations may trigger this
impl ExtensionList {
    /// Create a minimal extension list with only the base runtime, and no extensions
    #[must_use]
    pub fn new_minimal(options: ExtensionOptions) -> Self {
        let mut extensions = ExtensionList::new_empty(options);
        rustyscript::load(&mut extensions);
        extensions
    }

    /// Create a sandboxed extension list with only the base runtime, and all sandbox-preserving extensions
    #[must_use]
    pub fn new_sandboxed(options: ExtensionOptions) -> Self {
        let mut extensions = ExtensionList::new_minimal(options);

        #[cfg(feature = "webidl")]
        webidl::load(&mut extensions);

        #[cfg(all(not(feature = "web"), feature = "web_stub"))]
        web_stub::load(&mut extensions);

        #[cfg(feature = "console")]
        console::load(&mut extensions);

        #[cfg(feature = "url")]
        url::load(&mut extensions);

        #[cfg(all(feature = "crypto", all(not(feature = "web"), feature = "web_stub")))]
        crypto::load(&mut extensions);

        extensions
    }

    /// Creates an extension list with the given options, and all extensions defined by the active crate features
    #[must_use]
    pub fn new_default(options: ExtensionOptions) -> Self {
        let mut extensions = ExtensionList::new_sandboxed(options);

        #[cfg(feature = "web")]
        web::load(&mut extensions);

        #[cfg(all(feature = "crypto", feature = "web"))]
        crypto::load(&mut extensions);

        #[cfg(feature = "broadcast_channel")]
        broadcast_channel::load(&mut extensions);

        #[cfg(feature = "cache")]
        cache::load(&mut extensions);

        #[cfg(feature = "io")]
        io::load(&mut extensions);

        #[cfg(feature = "webstorage")]
        webstorage::load(&mut extensions);

        #[cfg(feature = "websocket")]
        websocket::load(&mut extensions);

        #[cfg(feature = "fs")]
        fs::load(&mut extensions);

        #[cfg(feature = "http")]
        http::load(&mut extensions);

        #[cfg(feature = "ffi")]
        ffi::load(&mut extensions);

        #[cfg(feature = "kv")]
        kv::load(&mut extensions);

        #[cfg(feature = "webgpu")]
        webgpu::load(&mut extensions);

        #[cfg(feature = "cron")]
        cron::load(&mut extensions);

        #[cfg(feature = "node_experimental")]
        {
            napi::load(&mut extensions);
            node::load(&mut extensions);
            runtime::load(&mut extensions);
        }

        extensions
    }
}
