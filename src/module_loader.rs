//! Module loader implementation for rustyscript
//! This module provides tools for caching module data, resolving module specifiers, and loading modules
#![allow(deprecated)]
use deno_core::{error::ModuleLoaderError, ModuleLoader, ModuleSpecifier};
use std::{borrow::Cow, cell::RefCell, path::PathBuf, rc::Rc};

mod cache_provider;
mod import_provider;
mod inner_loader;

use inner_loader::InnerRustyLoader;
pub(crate) use inner_loader::LoaderOptions;

// Public exports
pub use cache_provider::{ClonableSource, ModuleCacheProvider};
pub use import_provider::ImportProvider;

use crate::transpiler::ExtensionTranspiler;

/// The primary module loader implementation for rustyscript
/// This structure manages fetching module code, transpilation, and caching
pub(crate) struct RustyLoader {
    inner: Rc<RefCell<InnerRustyLoader>>,
}
impl RustyLoader {
    /// Creates a new instance of `RustyLoader`
    /// An optional cache provider can be provided to manage module code caching, as well as an import provider to manage module resolution.
    pub fn new(options: LoaderOptions) -> Self {
        let inner = Rc::new(RefCell::new(InnerRustyLoader::new(options)));
        Self { inner }
    }

    pub fn set_current_dir(&self, current_dir: PathBuf) {
        self.inner_mut().set_current_dir(current_dir);
    }

    fn inner(&self) -> std::cell::Ref<InnerRustyLoader> {
        self.inner.borrow()
    }

    fn inner_mut(&self) -> std::cell::RefMut<InnerRustyLoader> {
        self.inner.borrow_mut()
    }

    /// Inserts a source map into the source map cache
    /// This is used to provide source maps for loaded modules
    /// for error message generation
    pub fn insert_source_map(&self, file_name: &str, code: String, source_map: Option<Vec<u8>>) {
        self.inner_mut().add_source_map(file_name, code, source_map);
    }

    /// Get an extension transpiler that can be injected into a `deno_core::JsRuntime`
    pub fn as_extension_transpiler(self: &Rc<Self>) -> ExtensionTranspiler {
        let loader = self.clone();
        Rc::new(move |specifier, code| loader.inner().transpile_extension(&specifier, &code))
    }

    /// Transpile a module from CJS to ESM
    #[allow(dead_code)]
    pub async fn translate_cjs(
        &self,
        specifier: &ModuleSpecifier,
        source: &str,
    ) -> Result<String, crate::Error> {
        InnerRustyLoader::translate_cjs(self.inner.clone(), specifier.clone(), source.to_string())
            .await
    }
}

//
// Deno trait implementations start
//

impl ModuleLoader for RustyLoader {
    /// Resolve a module specifier to a full url by adding the base url
    /// and resolving any relative paths
    ///
    /// Also checks if the module is allowed to be loaded or not based on scheme
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        kind: deno_core::ResolutionKind,
    ) -> Result<ModuleSpecifier, ModuleLoaderError> {
        self.inner_mut().resolve(specifier, referrer, kind)
    }

    /// Load a module by it's name
    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        maybe_referrer: Option<&ModuleSpecifier>,
        is_dyn_import: bool,
        requested_module_type: deno_core::RequestedModuleType,
    ) -> deno_core::ModuleLoadResponse {
        let inner = self.inner.clone();
        InnerRustyLoader::load(
            inner,
            module_specifier,
            maybe_referrer,
            is_dyn_import,
            requested_module_type,
        )
    }

    fn get_source_map(&self, file_name: &str) -> Option<Cow<[u8]>> {
        let inner = self.inner();
        let map = inner.get_source_map(file_name)?.1.as_deref()?;
        Some(Cow::Owned(map.to_vec()))
    }

    fn get_source_mapped_source_line(&self, file_name: &str, line_number: usize) -> Option<String> {
        let inner = self.inner();
        let lines: Vec<_> = inner.get_source_map(file_name)?.0.split('\n').collect();
        if line_number >= lines.len() {
            return None;
        }
        Some(lines[line_number].to_string())
    }
}

#[cfg(test)]
mod test {
    use deno_core::{
        ModuleLoadResponse, ModuleSource, ModuleSourceCode, ModuleType, ResolutionKind,
    };

    use super::*;
    use crate::{module_loader::ClonableSource, traits::ToModuleSpecifier};

    /// Test in-memory module cache provider
    #[derive(Default)]
    struct MemoryModuleCacheProvider(std::collections::HashMap<ModuleSpecifier, ModuleSource>);
    impl ModuleCacheProvider for MemoryModuleCacheProvider {
        fn set(&mut self, specifier: &ModuleSpecifier, source: ModuleSource) {
            self.0.insert(specifier.clone(), source);
        }

        fn get(&self, specifier: &ModuleSpecifier) -> Option<ModuleSource> {
            self.0.get(specifier).map(|s| s.clone(specifier))
        }
    }

    #[tokio::test]
    async fn test_loader() {
        let mut cache_provider = MemoryModuleCacheProvider::default();
        let specifier = "file:///test.ts"
            .to_module_specifier(&std::env::current_dir().unwrap())
            .unwrap();
        let source = ModuleSource::new(
            ModuleType::JavaScript,
            ModuleSourceCode::String("console.log('Hello, World!')".to_string().into()),
            &specifier,
            None,
        );

        cache_provider.set(&specifier, source.clone(&specifier));
        let cached_source = cache_provider
            .get(&specifier)
            .expect("Expected to get cached source");

        let loader = RustyLoader::new(LoaderOptions {
            cache_provider: Some(Box::new(cache_provider)),
            ..LoaderOptions::default()
        });
        let response = loader.load(
            &specifier,
            None,
            false,
            deno_core::RequestedModuleType::None,
        );
        match response {
            ModuleLoadResponse::Async(_) => panic!("Unexpected response"),
            ModuleLoadResponse::Sync(result) => {
                let source = result.expect("Expected to get source");

                let ModuleSourceCode::String(source) = source.code else {
                    panic!("Unexpected source code type");
                };

                let ModuleSourceCode::String(cached_source) = cached_source.code else {
                    panic!("Unexpected source code type");
                };

                assert_eq!(source, cached_source);
            }
        }
    }

    struct TestImportProvider {
        i: usize,
    }
    impl TestImportProvider {
        fn new() -> Self {
            Self { i: 0 }
        }
    }
    impl ImportProvider for TestImportProvider {
        fn resolve(
            &mut self,
            specifier: &ModuleSpecifier,
            _referrer: &str,
            _kind: deno_core::ResolutionKind,
        ) -> Option<Result<ModuleSpecifier, ModuleLoaderError>> {
            match specifier.scheme() {
                "test" => {
                    self.i += 1;
                    Some(Ok(
                        ModuleSpecifier::parse(&format!("test://{}", self.i)).unwrap()
                    ))
                }
                _ => None,
            }
        }
        fn import(
            &mut self,
            specifier: &ModuleSpecifier,
            _referrer: Option<&ModuleSpecifier>,
            _is_dyn_import: bool,
            _requested_module_type: deno_core::RequestedModuleType,
        ) -> Option<Result<String, ModuleLoaderError>> {
            match specifier.as_str() {
                "test://1" => Some(Ok("console.log('Rock')".to_string())),
                "test://2" => Some(Ok("console.log('Paper')".to_string())),
                "test://3" => Some(Ok("console.log('Scissors')".to_string())),
                _ => None,
            }
        }
    }

    #[tokio::test]
    async fn test_import_provider() {
        let loader = RustyLoader::new(LoaderOptions {
            import_provider: Some(Box::new(TestImportProvider::new())),
            cwd: std::env::current_dir().unwrap(),
            ..LoaderOptions::default()
        });
        let expected_responses = [
            "console.log('Rock')".to_string(),
            "console.log('Paper')".to_string(),
            "console.log('Scissors')".to_string(),
        ];

        for expected in expected_responses {
            let specifier = loader
                .resolve("test://anything", "", ResolutionKind::Import)
                .unwrap();
            let response = loader.load(
                &specifier,
                None,
                false,
                deno_core::RequestedModuleType::None,
            );
            match response {
                ModuleLoadResponse::Async(future) => {
                    let source = future.await.expect("Expected to get source");
                    let ModuleSourceCode::String(source) = source.code else {
                        panic!("Unexpected source code type");
                    };

                    assert_eq!(source, expected.into());
                }

                ModuleLoadResponse::Sync(_) => panic!("Unexpected response"),
            }
        }
    }
}
