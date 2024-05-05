use deno_core::Extension;

pub mod rustyscript;

#[cfg(feature = "console")]
pub mod console;

#[cfg(feature = "crypto")]
pub mod crypto;

#[cfg(feature = "url")]
pub mod url;

#[cfg(feature = "web")]
pub mod web;

#[cfg(feature = "web_stub")]
pub mod web_stub;

#[cfg(feature = "webidl")]
pub mod webidl;

#[cfg(feature = "io")]
pub mod io;

///
/// Add up all required extensions
pub fn all_extensions(user_extensions: Vec<Extension>) -> Vec<Extension> {
    let mut extensions = rustyscript::extensions();

    #[cfg(feature = "console")]
    extensions.extend(console::extensions());

    #[cfg(feature = "webidl")]
    extensions.extend(webidl::extensions());

    #[cfg(feature = "url")]
    extensions.extend(url::extensions());

    #[cfg(feature = "web_stub")]
    extensions.extend(web_stub::extensions());

    #[cfg(feature = "web")]
    extensions.extend(web::extensions());

    #[cfg(feature = "crypto")]
    extensions.extend(crypto::extensions());

    #[cfg(feature = "io")]
    extensions.extend(io::extensions());

    extensions.extend(user_extensions);
    extensions
}
