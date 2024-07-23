//! This module provides a trait for caching module data for the loader
use deno_core::{
    ModuleCodeBytes, ModuleSource, ModuleSourceCode, ModuleSpecifier, SourceCodeCacheInfo,
};

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
/// Implement this trait to provide a custom module cache for the loader
/// The cache is used to store module data for later use, potentially saving time on re-fetching modules
#[deprecated(
    since = "0.7.0",
    note = "This trait is being replaced by the `ImportProvider` trait, which provides more control over module resolution. See the `module_loader_cache` example for more information."
)]
pub trait ModuleCacheProvider {
    /// Apply a module to the cache
    fn set(&mut self, specifier: &ModuleSpecifier, source: ModuleSource);

    /// Get a module from the cache
    fn get(&self, specifier: &ModuleSpecifier) -> Option<ModuleSource>;
}
