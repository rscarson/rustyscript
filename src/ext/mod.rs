use deno_core::Extension;

pub mod rustyscript;

#[macro_use]
mod mod_macros {
    macro_rules! import_mod {
        ($extensions_vec:ident, $mod:ident, $mod_path:literal, $includes:expr) => {
            #[path = $mod_path]
            pub mod $mod;
            $extensions_vec.extend($includes);
        };
    }
}

#[derive(Clone)]
pub struct Permissions;

#[cfg(feature = "web")]
impl deno_web::TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        true
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

    #[cfg(feature = "console")]
    import_mod!(
        extensions,
        init_console,
        "init_console.rs",
        vec![
            deno_console::deno_console::init_ops_and_esm(),
            init_console::init_console::init_ops_and_esm(),
        ]
    );

    #[cfg(feature = "webidl")]
    import_mod!(
        extensions,
        init_webidl,
        "init_webidl.rs",
        vec![
            deno_webidl::deno_webidl::init_ops_and_esm(),
            init_webidl::init_webidl::init_ops_and_esm(),
        ]
    );

    #[cfg(feature = "url")]
    import_mod!(
        extensions,
        init_url,
        "init_url.rs",
        vec![
            deno_url::deno_url::init_ops_and_esm(),
            init_url::init_url::init_ops_and_esm(),
        ]
    );

    #[cfg(feature = "crypto")]
    #[cfg(not(feature = "web"))]
    import_mod!(
        extensions,
        deno_web_stub,
        "deno_web_stub.rs",
        vec![deno_web_stub::deno_web::init_ops_and_esm(),]
    );

    #[cfg(feature = "web")]
    import_mod!(
        extensions,
        init_web,
        "init_web.rs",
        vec![
            deno_web::deno_web::init_ops_and_esm::<Permissions>(Default::default(), None),
            init_web::init_web::init_ops_and_esm(),
        ]
    );

    #[cfg(feature = "crypto")]
    import_mod!(
        extensions,
        init_crypto,
        "init_crypto.rs",
        vec![
            deno_crypto::deno_crypto::init_ops_and_esm(Default::default()),
            init_crypto::init_crypto::init_ops_and_esm(),
        ]
    );

    #[cfg(feature = "web")]
    import_mod!(
        extensions,
        init_fetch,
        "init_fetch.rs",
        vec![
            deno_fetch::deno_fetch::init_ops_and_esm::<Permissions>(Default::default()),
            init_fetch::init_fetch::init_ops_and_esm(),
        ]
    );

    extensions.extend(user_extensions);
    extensions
}

#[cfg(test)]
#[cfg(feature = "crypto")]
mod test_crypto {
    use crate::{Module, Runtime};

    #[test]
    fn test_crypto() {
        let module = Module::new(
            "test.js",
            "
            export const digest = crypto.getRandomValues(new Uint32Array(10)).toString();
            ",
        );

        let mut runtime = Runtime::new(Default::default()).expect("could not create runtime");
        let module_handle = runtime.load_module(&module).expect("could not load module");
        let value: String = runtime
            .get_value(&module_handle, "digest")
            .expect("could not get value");
        assert_eq!(value.split(",").collect::<Vec<&str>>().len(), 10);
    }
}
