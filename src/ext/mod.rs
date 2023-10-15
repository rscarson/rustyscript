use deno_core::Extension;

pub mod rustyscript;

#[macro_use]
mod mod_macros {
    macro_rules! import_mod {
        ($extensions_vec:ident, $mod:ident, $mod_path:literal, $feature:literal, $includes:expr) => {
            #[cfg(feature = $feature)]
            #[path = $mod_path]
            pub mod $mod;

            #[cfg(feature = $feature)]
            $extensions_vec.extend($includes);
        };
    }
}

#[derive(Clone)]
#[cfg(feature = "web")]
struct Permissions;

#[cfg(feature = "web")]
impl deno_web::TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        true
    }
    fn check_unstable(&self, _state: &deno_core::OpState, _api_name: &'static str) {
        unreachable!()
    }
}

#[cfg(feature = "web")]
impl deno_fetch::FetchPermissions for Permissions {
    fn check_net_url(
        &mut self,
        _url: &deno_core::url::Url,
        _api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }

    fn check_read(
        &mut self,
        _p: &std::path::Path,
        _api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }
}

/// Adds internal extensions to the list provided by the user
///
/// # Arguments
/// * `user_extensions` - A set of deno_core::Extension objects provided by the user
pub fn all_extensions(user_extensions: Vec<Extension>) -> Vec<Extension> {
    let mut extensions: Vec<Extension> = vec![rustyscript::rustyscript::init_ops_and_esm()];

    import_mod!(
        extensions,
        init_console,
        "init_console.rs",
        "console",
        vec![
            deno_console::deno_console::init_ops_and_esm(),
            init_console::init_console::init_ops_and_esm(),
        ]
    );

    import_mod!(
        extensions,
        init_webidl,
        "init_webidl.rs",
        "webidl",
        vec![
            deno_webidl::deno_webidl::init_ops_and_esm(),
            init_webidl::init_webidl::init_ops_and_esm(),
        ]
    );

    import_mod!(
        extensions,
        init_url,
        "init_url.rs",
        "url",
        vec![
            deno_url::deno_url::init_ops_and_esm(),
            init_url::init_url::init_ops_and_esm(),
        ]
    );

    import_mod!(
        extensions,
        init_web,
        "init_web.rs",
        "web",
        vec![
            deno_web::deno_web::init_ops_and_esm::<Permissions>(Default::default(), None),
            init_web::init_web::init_ops_and_esm(),
        ]
    );

    import_mod!(
        extensions,
        init_crypto,
        "init_crypto.rs",
        "web",
        vec![
            deno_crypto::deno_crypto::init_ops_and_esm(Default::default()),
            init_crypto::init_crypto::init_ops_and_esm(),
        ]
    );

    import_mod!(
        extensions,
        init_fetch,
        "init_fetch.rs",
        "web",
        vec![
            deno_fetch::deno_fetch::init_ops_and_esm::<Permissions>(Default::default()),
            init_fetch::init_fetch::init_ops_and_esm(),
        ]
    );

    extensions.extend(user_extensions);
    extensions
}
