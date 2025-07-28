use super::ExtensionTrait;
use deno_core::{extension, Extension};

mod simple;
pub use simple::{sqlite_cache, temp_cache};

extension!(
    init_cache,
    deps = [rustyscript],
    esm_entry_point = "ext:init_cache/init_cache.js",
    esm = [ dir "src/ext/cache", "init_cache.js" ],
);
impl ExtensionTrait<()> for init_cache {
    fn init((): ()) -> Extension {
        init_cache::init()
    }
}
impl ExtensionTrait<Option<deno_cache::CreateCache>> for deno_cache::deno_cache {
    fn init(options: Option<deno_cache::CreateCache>) -> Extension {
        deno_cache::deno_cache::init(options)
    }
}

pub fn extensions(
    options: Option<deno_cache::CreateCache>,
    is_snapshot: bool,
) -> Vec<Extension> {
    vec![
        deno_cache::deno_cache::build(options, is_snapshot),
        init_cache::build((), is_snapshot),
    ]
}

#[cfg(test)]
mod test {
    use crate::{Module, Runtime, RuntimeOptions};

    #[test]
    fn test_default_mem_cache() {
        let mut runtime = Runtime::new(RuntimeOptions::default()).unwrap();
        let module = Module::new(
            "test.js",
            "
                let cache = await caches.open('my_cache');

                fetch('http://web.simmons.edu/').then((response) => {
                    cache.put('http://web.simmons.edu/', response);
                });

                cache.match('http://web.simmons.edu/').then((response) => {
                    console.log('Got response from cache!');
                });
            ",
        );

        runtime.load_module(&module).unwrap();
    }
}
