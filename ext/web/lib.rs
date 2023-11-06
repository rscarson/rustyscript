use deno_core::{extension, Extension};

#[derive(Clone)]
pub struct Permissions;

impl deno_web::TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        true
    }
}

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

extension!(
    init_web,
    deps = [rustyscript],
    esm_entry_point = "ext:init_web/init_web.js",
    esm = [ dir ".", "init_web.js" ],
    state = |state| state.put(Permissions{})
);

extension!(
    init_fetch,
    deps = [rustyscript],
    esm_entry_point = "ext:init_fetch/init_fetch.js",
    esm = [ dir ".", "init_fetch.js" ],
    state = |state| state.put(Permissions{})
);

pub fn extensions() -> Vec<Extension> {
    vec![
        deno_web::deno_web::init_ops_and_esm::<Permissions>(Default::default(), None),
        deno_fetch::deno_fetch::init_ops_and_esm::<Permissions>(Default::default()),
        init_web::init_ops_and_esm(),
        init_fetch::init_ops_and_esm(),
    ]
}
