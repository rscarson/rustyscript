use crate::{
    cache_provider::{ClonableSource, ModuleCacheProvider},
    transpiler,
};
use deno_core::{
    anyhow::{self, anyhow},
    futures::FutureExt,
    ModuleLoadResponse, ModuleLoader, ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType,
    SourceMapGetter,
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

/// Stores the source code and source map for loaded modules
type SourceMapCache = HashMap<String, (String, Option<Vec<u8>>)>;

/// Internal implementation ModuleLoader
#[derive(Clone)]
struct InnerRustyLoader {
    cache_provider: Rc<Option<Box<dyn ModuleCacheProvider>>>,
    fs_whlist: Rc<RefCell<HashSet<String>>>,
    source_map_cache: Rc<RefCell<SourceMapCache>>,
}

impl InnerRustyLoader {
    /// Creates a new instance of InnerRustyLoader
    /// An optional cache provider can be provided to manage module code caching
    fn new(cache_provider: Option<Box<dyn ModuleCacheProvider>>) -> Self {
        Self {
            cache_provider: Rc::new(cache_provider),
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
        let cache_provider = cache_provider.as_ref().as_ref().map(|p| p.as_ref());
        match cache_provider.map(|p| p.get(&module_specifier)) {
            Some(Some(source)) => Ok(source),
            _ => {
                // Not in the cache, load the module from the handler

                // Get the module type first
                let module_type = if module_specifier.path().ends_with(".json") {
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
        _kind: deno_core::ResolutionKind,
    ) -> Result<ModuleSpecifier, anyhow::Error> {
        // Resolve the module specifier to an absolute URL
        let url = deno_core::resolve_import(specifier, referrer)?;
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
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
        _requested_module_type: deno_core::RequestedModuleType,
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
                                .map_err(|_| anyhow!("`{specifier}` is not a valid file URL."))?;
                            Ok(tokio::fs::read_to_string(path).await?)
                        })
                        .await
                }
                .boxed_local(),
            ),

            // Unknown scheme - deny
            _ => ModuleLoadResponse::Sync(Err(anyhow!(
                "{} imports are not allowed here: {}",
                module_specifier.scheme(),
                module_specifier.as_str()
            ))),
        }
    }
}

#[allow(dead_code)]
impl RustyLoader {
    /// Creates a new instance of RustyLoader
    /// An optional cache provider can be provided to manage module code caching
    pub fn new(cache_provider: Option<Box<dyn ModuleCacheProvider>>) -> Self {
        Self {
            inner: Rc::new(InnerRustyLoader::new(cache_provider)),
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
        sref.1.as_ref().map(|s| s.to_vec())
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
        let specifier = "file:///test.ts".to_module_specifier().unwrap();
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

        let loader = RustyLoader::new(Some(Box::new(cache_provider)));
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
                let cached_source = if let ModuleSourceCode::String(s) = cached_source.code {
                    s
                } else {
                    panic!("Unexpected source code type");
                };
                assert_eq!(source, cached_source);
            }
            _ => panic!("Unexpected response"),
        }
    }
}
