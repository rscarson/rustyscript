//! This module provides a trait for caching module data for the loader
use deno_core::{
    ModuleCodeBytes, ModuleSource, ModuleSourceCode, ModuleSpecifier, SourceCodeCacheInfo,
};
use std::{cell::RefCell, collections::HashMap};

/// A helper trait to clone a `ModuleSource`
/// `deno_core::ModuleSource` does not implement Clone, so we need to implement it ourselves
/// for our cache providers to work
///
/// Todo: This is a temporary solution, we should submit a PR to `deno_core` to implement Clone for `ModuleSource`
pub trait ClonableSource {
    /// Create a new copy of a `ModuleSource`
    fn clone(&self, specifier: &ModuleSpecifier) -> ModuleSource;
}
impl ClonableSource for ModuleSource {
    fn clone(&self, specifier: &ModuleSpecifier) -> ModuleSource {
        ModuleSource::new(
            self.module_type.clone(),
            match &self.code {
                ModuleSourceCode::String(s) => ModuleSourceCode::String(s.to_string().into()),
                ModuleSourceCode::Bytes(b) => {
                    ModuleSourceCode::Bytes(ModuleCodeBytes::Boxed(b.to_vec().into()))
                }
            },
            specifier,
            self.code_cache.as_ref().map(|c| SourceCodeCacheInfo {
                hash: c.hash,
                data: c.data.clone(),
            }),
        )
    }
}

/// Module cache provider trait
/// Implement this trait to provide a custom module cache
/// You will need to use interior due to the deno's loader trait
/// Default cache for the loader is in-memory
pub trait ModuleCacheProvider {
    /// Apply a module to the cache
    fn set(&self, specifier: &ModuleSpecifier, source: ModuleSource);

    /// Get a module from the cache
    fn get(&self, specifier: &ModuleSpecifier) -> Option<ModuleSource>;
}

/// Default in-memory module cache provider
#[derive(Default)]
pub struct MemoryModuleCacheProvider(RefCell<HashMap<ModuleSpecifier, ModuleSource>>);
impl ModuleCacheProvider for MemoryModuleCacheProvider {
    fn set(&self, specifier: &ModuleSpecifier, source: ModuleSource) {
        self.0.borrow_mut().insert(specifier.clone(), source);
    }

    fn get(&self, specifier: &ModuleSpecifier) -> Option<ModuleSource> {
        let cache = self.0.borrow();
        let source = cache.get(specifier)?;
        Some(source.clone(specifier))
    }
}
