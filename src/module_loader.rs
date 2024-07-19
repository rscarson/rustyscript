use crate::{
    cache_provider::{ClonableSource, ModuleCacheProvider},
    transpiler,
};
use deno_core::{
    anyhow::{self, anyhow, Ok},
    futures::FutureExt,
    ModuleLoadResponse, ModuleLoader, ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType,
    SourceMapGetter,
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
};

#[allow(unused_variables)]
#[cfg(feature = "import_provider")]
/// A trait that can be implemented to provide custom import resolution. Passed to the runtime via `RuntimeOptions::import_provider`
pub trait ImportProvider {
    /// Resolve an import statement's specifier to a URL to later be imported
    fn resolve(
        &mut self,
        specifier: &ModuleSpecifier,
        referrer: &str,
        kind: deno_core::ResolutionKind,
    ) -> Option<Result<ModuleSpecifier, anyhow::Error>> {
        None
    }
    /// Retrieve a JavaScript/TypeScript module from a given URL and return it as a string.
    fn import(
        &mut self,
        specifier: &ModuleSpecifier,
        referrer: &Option<ModuleSpecifier>,
        is_dyn_import: bool,
        requested_module_type: deno_core::RequestedModuleType,
    ) -> Option<Result<String, anyhow::Error>> {
        None
    }
}

/// Stores the source code and source map for loaded modules
type SourceMapCache = HashMap<String, (String, Option<Vec<u8>>)>;

/// Internal implementation `ModuleLoader`
#[derive(Clone)]
struct InnerRustyLoader {
    cache_provider: Rc<Option<Box<dyn ModuleCacheProvider>>>,
    #[cfg(feature = "import_provider")]
    import_provider: Rc<Option<RefCell<Box<dyn ImportProvider>>>>,
    fs_whlist: Rc<RefCell<HashSet<String>>>,
    source_map_cache: Rc<RefCell<SourceMapCache>>,
}

impl InnerRustyLoader {
    /// Creates a new instance of `InnerRustyLoader`
    /// An optional cache provider can be provided to manage module code caching, as well as an import provider to manage module resolution.
    fn new(
        cache_provider: Option<Box<dyn ModuleCacheProvider>>,
        #[cfg(feature = "import_provider")] import_provider: Option<Box<dyn ImportProvider>>,
    ) -> Self {
        Self {
            cache_provider: Rc::new(cache_provider),
            #[cfg(feature = "import_provider")]
            import_provider: Rc::new(import_provider.map(RefCell::new)),
            fs_whlist: Rc::new(RefCell::new(HashSet::new())),
            source_map_cache: Rc::new(RefCell::new(SourceMapCache::new())),
        }
    }

    /// Adds a module specifier to the whitelist
    /// This allows the module to be loaded from the filesystem
    /// If they are included from rust first
    fn whitelist_add(&self, specifier: &str) {
        self.fs_whlist.borrow_mut().insert(specifier.to_string());
    }

    /// Checks if a module specifier is in the whitelist
    /// Used to determine if a module can be loaded from the filesystem
    /// or not if `fs_import` is disabled
    fn whitelist_has(&self, specifier: &str) -> bool {
        self.fs_whlist.borrow_mut().contains(specifier)
    }

    /// Loads a module's source code from the cache or from the provided handler
    async fn load<F, Fut>(
        &self,
        module_specifier: ModuleSpecifier,
        handler: F,
    ) -> Result<ModuleSource, deno_core::error::AnyError>
    where
        F: Fn(ModuleSpecifier) -> Fut,
        Fut: std::future::Future<Output = Result<String, deno_core::error::AnyError>>,
    {
        // Check if the module is in the cache first
        let cache_provider = self.cache_provider.clone();
        let cache_provider = cache_provider.as_ref().as_ref().map(AsRef::as_ref);
        if let Some(Some(source)) = cache_provider.map(|p| p.get(&module_specifier)) {
            Ok(source)
        } else {
            // Not in the cache, load the module from the handler

            // Get the module type first
            let extension = Path::new(module_specifier.path())
                .extension()
                .unwrap_or_default();
            let module_type = if extension.eq_ignore_ascii_case("json") {
                ModuleType::Json
            } else {
                ModuleType::JavaScript
            };

            // Load the module code, and transpile it if necessary
            let code = handler(module_specifier.clone()).await?;
            let (tcode, source_map) = transpiler::transpile(&module_specifier, &code)?;

            // Create the module source
            let source = ModuleSource::new(
                module_type,
                ModuleSourceCode::String(tcode.into()),
                &module_specifier,
                None,
            );

            // Add the source to our source cache
            self.source_map_cache.borrow_mut().insert(
                module_specifier.to_string(),
                (code, source_map.map(|s| s.to_vec())),
            );

            // Cache the source if a cache provider is available
            // Could speed up loads on some future runtime
            if let Some(p) = cache_provider {
                p.set(&module_specifier, source.clone(&module_specifier));
            }
            Ok(source)
        }
    }

    /// Returns a reference to the source map cache
    fn source_map_cache(&self) -> Rc<RefCell<SourceMapCache>> {
        self.source_map_cache.clone()
    }
}

/// The primary module loader implementation for rustyscript
/// This structure manages fetching module code, transpilation, and caching
pub struct RustyLoader {
    inner: Rc<InnerRustyLoader>,
}
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
    ) -> Result<ModuleSpecifier, anyhow::Error> {
        // Resolve the module specifier to an absolute URL
        let url = deno_core::resolve_import(specifier, referrer)?;

        #[cfg(feature = "import_provider")]
        if let Some(import_provider) = self.inner.import_provider.as_ref().as_ref() {
            let resolve_result = import_provider.borrow_mut().resolve(&url, referrer, kind);

            // ImportProvider's resolve method should return None if default resolution is preferred, and Some(Err) if a url is not allowed
            if let Some(result) = resolve_result {
                return result;
            }
        }

        if referrer == "." {
            // Added from rust, add to the whitelist
            // so we can load it from the filesystem
            self.whitelist_add(url.as_str());
        }

        // We check permissions first
        match url.scheme() {
            // Remote fetch imports
            "https" | "http" => {
                #[cfg(not(feature = "url_import"))]
                return Err(anyhow!("web imports are not allowed here: {specifier}"));
            }

            // Dynamic FS imports
            "file" =>
            {
                #[cfg(not(feature = "fs_import"))]
                if !self.whitelist_has(url.as_str()) {
                    return Err(anyhow!("requested module is not loaded: {specifier}"));
                }
            }

            _ if specifier.starts_with("ext:") => {
                // Extension import - allow
            }

            _ => {
                return Err(anyhow!(
                    "unrecognized schema for module import: {specifier}"
                ));
            }
        }
        Ok(url)
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
        let module_specifier = module_specifier.clone();
        // We check permissions first
        match module_specifier.scheme() {
            // Remote fetch imports
            #[cfg(feature = "url_import")]
            "https" | "http" => ModuleLoadResponse::Async(
                async move {
                    inner
                        .load(module_specifier, |specifier| async move {
                            let response = reqwest::get(specifier).await?;
                            Ok(response.text().await?)
                        })
                        .await
                }
                .boxed_local(),
            ),

            // FS imports
            "file" => ModuleLoadResponse::Async(
                async move {
                    inner
                        .load(module_specifier, |specifier| async move {
                            let path = specifier
                                .to_file_path()
                                .map_err(|()| anyhow!("`{specifier}` is not a valid file URL."))?;
                            Ok(tokio::fs::read_to_string(path).await?)
                        })
                        .await
                }
                .boxed_local(),
            ),

            _ => {
                #[cfg(feature = "import_provider")]
                if inner.import_provider.is_some() {
                    let maybe_referrer = Rc::new(maybe_referrer.cloned());
                    return ModuleLoadResponse::Async(
                        async move {
                            inner
                                .load(module_specifier, |specifier| {
                                    let import_provider =
                                        inner.import_provider.as_ref().as_ref().unwrap();
                                    let maybe_referrer = maybe_referrer.as_ref();
                                    let requested_module_type = requested_module_type.clone();
                                    async move {
                                        let import = import_provider.borrow_mut().import(
                                            &specifier,
                                            maybe_referrer,
                                            is_dyn_import,
                                            requested_module_type,
                                        );
                                        if let Some(import) = import {
                                            import
                                        } else {
                                            Err(anyhow!(
                                                "{} imports are not allowed here: {}",
                                                specifier.scheme(),
                                                specifier.as_str()
                                            ))
                                        }
                                    }
                                })
                                .await
                        }
                        .boxed_local(),
                    );
                }

                // Unknown scheme - deny
                return ModuleLoadResponse::Sync(Err(anyhow!(
                    "{} imports are not allowed here: {}",
                    module_specifier.scheme(),
                    module_specifier.as_str()
                )));
            }
        }
    }
}

#[allow(dead_code)]
impl RustyLoader {
    /// Creates a new instance of `RustyLoader`
    /// An optional cache provider can be provided to manage module code caching, as well as an import provider to manage module resolution.
    pub fn new(
        cache_provider: Option<Box<dyn ModuleCacheProvider>>,
        #[cfg(feature = "import_provider")] import_provider: Option<Box<dyn ImportProvider>>,
    ) -> Self {
        Self {
            inner: Rc::new(InnerRustyLoader::new(
                cache_provider,
                #[cfg(feature = "import_provider")]
                import_provider,
            )),
        }
    }

    /// Adds a module specifier to the whitelist
    /// This allows the module to be loaded from the filesystem
    /// If they are included from rust first when `fs_import` is disabled
    pub fn whitelist_add(&self, specifier: &str) {
        self.inner.whitelist_add(specifier);
    }

    /// Checks if a module specifier is in the whitelist
    /// Used to determine if a module can be loaded from the filesystem
    /// or not if `fs_import` is disabled
    pub fn whitelist_has(&self, specifier: &str) -> bool {
        self.inner.whitelist_has(specifier)
    }

    /// Inserts a source map into the source map cache
    /// This is used to provide source maps for loaded modules
    /// for error message generation
    pub fn insert_source_map(&self, file_name: &str, code: String, source_map: Option<Vec<u8>>) {
        self.inner
            .source_map_cache
            .borrow_mut()
            .insert(file_name.to_string(), (code, source_map));
    }
}

impl SourceMapGetter for RustyLoader {
    /// Gets the source map for a loaded module by name
    /// Used for error generation for modules that were transpiled
    fn get_source_map(&self, file_name: &str) -> Option<Vec<u8>> {
        let sref = self.inner.source_map_cache();
        let sref = sref.borrow();
        let sref = sref.get(file_name)?;
        sref.1.clone()
    }

    /// Get a specific line from a source file in the cache
    fn get_source_line(&self, file_name: &str, line_number: usize) -> Option<String> {
        let map = self.inner.source_map_cache();
        let map = map.borrow();
        let code = map.get(file_name).map(|(c, _)| c)?;
        let lines: Vec<&str> = code.split('\n').collect();
        if line_number >= lines.len() {
            return None;
        }
        Some(lines[line_number].to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        cache_provider::{ClonableSource, MemoryModuleCacheProvider},
        traits::ToModuleSpecifier,
    };

    #[tokio::test]
    async fn test_loader() {
        let cache_provider = MemoryModuleCacheProvider::default();
        let specifier = "file:///test.ts".to_module_specifier(None).unwrap();
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

        let loader = RustyLoader::new(
            Some(Box::new(cache_provider)),
            #[cfg(feature = "import_provider")]
            None,
        );
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

                let ModuleSourceCode::String(cached_source) = cached_source.code else {
                    panic!("Unexpected source code type");
                };

                assert_eq!(source, cached_source);
            }
            ModuleLoadResponse::Sync(_) => panic!("Unexpected response"),
        }
    }

    #[cfg(feature = "import_provider")]
    use deno_core::ResolutionKind;

    #[cfg(feature = "import_provider")]
    struct TestImportProvider {
        i: usize,
    }
    #[cfg(feature = "import_provider")]
    impl TestImportProvider {
        fn new() -> Self {
            Self { i: 0 }
        }
    }
    #[cfg(feature = "import_provider")]
    impl ImportProvider for TestImportProvider {
        fn resolve(
            &mut self,
            specifier: &ModuleSpecifier,
            _referrer: &str,
            _kind: deno_core::ResolutionKind,
        ) -> Option<Result<ModuleSpecifier, deno_core::anyhow::Error>> {
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
            _referrer: &Option<ModuleSpecifier>,
            _is_dyn_import: bool,
            _requested_module_type: deno_core::RequestedModuleType,
        ) -> Option<Result<String, deno_core::anyhow::Error>> {
            match specifier.as_str() {
                "test://1" => Some(Ok("console.log('Rock')".to_string())),
                "test://2" => Some(Ok("console.log('Paper')".to_string())),
                "test://3" => Some(Ok("console.log('Scissors')".to_string())),
                _ => None,
            }
        }
    }

    #[tokio::test]
    #[cfg(feature = "import_provider")]
    async fn test_import_provider() {
        let loader = RustyLoader::new(None, Some(Box::new(TestImportProvider::new())));
        let expected_responses = vec![
            "console.log('Rock')".to_string(),
            "console.log('Paper')".to_string(),
            "console.log('Scissors')".to_string(),
        ];
        for i in 0..3 {
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
                    let source = if let ModuleSourceCode::String(s) = source.code {
                        s
                    } else {
                        panic!("Unexpected source code type");
                    };
                    assert_eq!(source, expected_responses[i].clone().into());
                }
                _ => panic!("Unexpected response"),
            }
        }
    }
}
