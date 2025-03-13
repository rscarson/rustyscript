//!
//! This example will demonstrate usage of the `ImportProvider` trait to implement a cache for module loading.
//! This one will be a simple in-memory cache
//!
use deno_core::{error::ModuleLoaderError, ModuleSource, ModuleSpecifier};
use rustyscript::{module_loader::ImportProvider, Module, Runtime, RuntimeOptions};
use std::collections::HashMap;

/// A simple in-memory cache for module loading
#[derive(Default)]
pub struct MemoryCache {
    cache: HashMap<String, String>,
}
impl MemoryCache {
    /// Set a module in the cache
    pub fn set(&mut self, specifier: &str, source: String) {
        self.cache.insert(specifier.to_string(), source);
    }

    /// Get a module from the cache
    pub fn get(&self, specifier: &ModuleSpecifier) -> Option<String> {
        self.cache.get(specifier.as_str()).cloned()
    }

    pub fn has(&self, specifier: &ModuleSpecifier) -> bool {
        self.cache.contains_key(specifier.as_str())
    }
}

impl ImportProvider for MemoryCache {
    fn resolve(
        &mut self,
        specifier: &ModuleSpecifier,
        _referrer: &str,
        _kind: deno_core::ResolutionKind,
    ) -> Option<Result<ModuleSpecifier, ModuleLoaderError>> {
        // Tell the loader to allow the import if the module is in the cache
        self.get(specifier).map(|_| Ok(specifier.clone()))
    }

    fn import(
        &mut self,
        specifier: &ModuleSpecifier,
        _referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
        _requested_module_type: deno_core::RequestedModuleType,
    ) -> Option<Result<String, ModuleLoaderError>> {
        // Return the source code if the module is in the cache
        self.get(specifier).map(Ok)
    }

    fn post_process(
        &mut self,
        specifier: &ModuleSpecifier,
        source: ModuleSource,
    ) -> Result<ModuleSource, ModuleLoaderError> {
        // Cache the source code
        if !self.has(specifier) {
            match &source.code {
                deno_core::ModuleSourceCode::String(s) => {
                    self.set(specifier.as_str(), s.to_string());
                }
                deno_core::ModuleSourceCode::Bytes(_) => {}
            }
        }
        Ok(source)
    }
}

fn main() -> Result<(), rustyscript::Error> {
    let mut cache = MemoryCache::default();
    cache.set(
        "http://example.com/my_module.js",
        "export const foo = 'bar';".to_string(),
    );

    let mut runtime = Runtime::new(RuntimeOptions {
        import_provider: Some(Box::new(cache)),
        ..Default::default()
    })?;

    let module = Module::new(
        "example.js",
        "
        import { foo } from 'http://example.com/my_module.js';
        if (foo !== 'bar') {
            throw new Error('Expected foo to be bar');
        }
    ",
    );

    runtime.load_module(&module)?;
    Ok(())
}
