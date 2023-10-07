use deno_core::Extension;

#[cfg(feature = "console")]
pub mod init_console;

#[cfg(feature = "url")]
pub mod init_url;

#[cfg(feature = "web")]
pub mod init_web;

pub mod js_playground;

#[cfg(feature = "web")]
#[derive(Clone)]
struct Permissions;
#[cfg(feature = "web")]
impl deno_web::TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        false
    }
    fn check_unstable(&self, _state: &deno_core::OpState, _api_name: &'static str) {
        unreachable!()
    }
}

/// Adds internal extensions to the list provided by the user
///
/// # Arguments
/// * `user_extensions` - A set of deno_core::Extension objects provided by the user
pub fn all_extensions(mut user_extensions: Vec<Extension>) -> Vec<Extension> {
    user_extensions.extend(vec![js_playground::js_playground::init_ops_and_esm()]);

    #[cfg(feature = "console")]
    user_extensions.extend(vec![
        deno_console::deno_console::init_ops_and_esm(),
        crate::ext::init_console::init_console::init_ops_and_esm(),
    ]);

    #[cfg(feature = "url")]
    user_extensions.extend(vec![
        deno_webidl::deno_webidl::init_ops_and_esm(),
        deno_url::deno_url::init_ops_and_esm(),
        crate::ext::init_url::init_url::init_ops_and_esm(),
    ]);

    #[cfg(feature = "web")]
    user_extensions.extend(vec![
        deno_web::deno_web::init_ops_and_esm::<Permissions>(Default::default(), None),
        crate::ext::init_web::init_web::init_ops_and_esm(),
    ]);

    user_extensions
}
