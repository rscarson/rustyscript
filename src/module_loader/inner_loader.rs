#![allow(unused_imports)]
#![allow(deprecated)]
#![allow(dead_code)]
use crate::module_loader::{ClonableSource, ModuleCacheProvider};
use crate::traits::ToModuleSpecifier;
use crate::transpiler::{transpile, transpile_extension, ExtensionTranspilation};
use deno_core::anyhow::{anyhow, Error};
use deno_core::error::AnyError;
use deno_core::futures::FutureExt;
use deno_core::{
    FastString, ModuleLoadResponse, ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

#[cfg(feature = "node_experimental")]
use crate::ext::node::NodeCodeTranslator;
#[cfg(feature = "node_experimental")]
use crate::ext::node::RustyResolver;
#[cfg(feature = "node_experimental")]
use deno_node::NodeResolver;
#[cfg(feature = "node_experimental")]
use node_resolver::InNpmPackageChecker;
#[cfg(feature = "node_experimental")]
use node_resolver::{NodeModuleKind, NodeResolutionMode};

use super::ImportProvider;

/// Stores the source code and source ma#![allow(deprecated)]p for loaded modules
type SourceMapCache = HashMap<String, (String, Option<Vec<u8>>)>;

/// Options for the `RustyLoader` struct
/// Not for public use
#[derive(Default)]
pub struct LoaderOptions {
    /// An optional cache provider to manage module code caching
    pub cache_provider: Option<Box<dyn ModuleCacheProvider>>,

    /// A whitelist of module specifiers that are always allowed to be loaded from the filesystem
    pub fs_whitelist: HashSet<String>,

    /// A cache for source maps for loaded modules
    /// Used for error message generation
    pub source_map_cache: SourceMapCache,

    /// A resolver for node modules
    #[cfg(feature = "node_experimental")]
    pub node_resolver: Arc<RustyResolver>,

    /// An optional import provider to manage module resolution
    pub import_provider: Option<Box<dyn ImportProvider>>,

    /// A whitelist of custom schema prefixes that are allowed to be loaded
    pub schema_whlist: HashSet<String>,

    /// The current working directory for the loader
    pub cwd: PathBuf,
}

#[cfg(feature = "node_experimental")]
struct NodeProvider {
    rusty_resolver: Arc<RustyResolver>,
    node_resolver: Arc<NodeResolver>,
    code_translator: Rc<NodeCodeTranslator>,
}
#[cfg(feature = "node_experimental")]
impl NodeProvider {
    pub fn new(resolver: Arc<RustyResolver>) -> Self {
        let node_resolver = Arc::new(resolver.node_resolver());
        let code_translator = Rc::new(resolver.code_translator(node_resolver.clone()));
        Self {
            rusty_resolver: resolver,
            node_resolver,
            code_translator,
        }
    }
}

/// Internal implementation of the module loader
/// Stores the cache provider, filesystem whitelist, and source map cache
/// Unlike the outer loader, this struture does not need to rely on inner mutability
///
/// Not for public use
pub struct InnerRustyLoader {
    cache_provider: Option<Box<dyn ModuleCacheProvider>>,
    fs_whlist: HashSet<String>,
    source_map_cache: SourceMapCache,
    import_provider: Option<Box<dyn ImportProvider>>,
    schema_whlist: HashSet<String>,
    cwd: PathBuf,

    #[cfg(feature = "node_experimental")]
    node: NodeProvider,
}

impl InnerRustyLoader {
    /// Creates a new instance of `InnerRustyLoader`
    /// An optional cache provider can be provided to manage module code caching, as well as an import provider to manage module resolution.
    pub fn new(options: LoaderOptions) -> Self {
        Self {
            cache_provider: options.cache_provider,
            fs_whlist: options.fs_whitelist,
            source_map_cache: options.source_map_cache,
            import_provider: options.import_provider,
            schema_whlist: options.schema_whlist,
            cwd: options.cwd,

            #[cfg(feature = "node_experimental")]
            node: NodeProvider::new(options.node_resolver),
        }
    }

    /// Sets the current working directory for the loader
    pub fn set_current_dir(&mut self, cwd: PathBuf) {
        self.cwd = cwd;
    }

    /// Adds a module specifier to the whitelist
    /// This allows the module to be loaded from the filesystem
    /// If they are included from rust first
    pub fn whitelist_add(&mut self, specifier: &str) {
        self.fs_whlist.insert(specifier.to_string());
    }

    /// Checks if a module specifier is in the whitelist
    /// Used to determine if a module can be loaded from the filesystem
    /// or not if `fs_import` is disabled
    pub fn whitelist_has(&self, specifier: &str) -> bool {
        self.fs_whlist.contains(specifier)
    }

    #[allow(clippy::unused_self)]
    pub fn transpile_extension(
        &self,
        specifier: &FastString,
        code: &FastString,
    ) -> Result<ExtensionTranspilation, AnyError> {
        let specifier = specifier.as_str().to_module_specifier(&self.cwd)?;
        let code = code.as_str();
        transpile_extension(&specifier, code)
    }

    pub fn resolve(
        &mut self,
        specifier: &str,
        referrer: &str,
        kind: deno_core::ResolutionKind,
    ) -> Result<ModuleSpecifier, Error> {
        //
        // Handle import aliasing for node imports
        #[cfg(feature = "node_experimental")]
        if specifier.starts_with('#') {
            let referrer = if deno_core::specifier_has_uri_scheme(referrer) {
                deno_core::resolve_url(referrer)?
            } else {
                referrer.to_module_specifier(&self.cwd)?
            };

            let package = self
                .node
                .rusty_resolver
                .package_json_resolver()
                .get_closest_package_json(&referrer)?;
            if let Some(package) = package {
                let url = self.node.node_resolver.package_imports_resolve(
                    specifier,
                    Some(&referrer),
                    NodeModuleKind::Esm,
                    Some(&package),
                    node_resolver::REQUIRE_CONDITIONS,
                    NodeResolutionMode::Execution,
                )?;
                return Ok(url);
            }
        }

        // Resolve the module specifier to an absolute URL
        let url = deno_core::resolve_import(specifier, referrer)?;

        // Check if the module is in the cache
        if self
            .cache_provider
            .as_ref()
            .is_some_and(|c| c.get(&url).is_some())
        {
            return Ok(url);
        }

        // Check if the import provider allows the import
        if let Some(import_provider) = &mut self.import_provider {
            let resolve_result = import_provider.resolve(&url, referrer, kind);
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

            #[cfg(feature = "node_experimental")]
            _ if specifier.starts_with("npm:") || specifier.starts_with("node:") => {
                let referrer = if deno_core::specifier_has_uri_scheme(referrer) {
                    deno_core::resolve_url(referrer)?
                } else {
                    referrer.to_module_specifier(&self.cwd)?
                };

                // Strip the scheme from the specifier
                let specifier_ = &specifier[specifier.find(':').unwrap()..];
                let specifier = if specifier.len() == 1 {
                    return Err(anyhow!("invalid node specifier: {specifier}"));
                } else {
                    specifier_[1..].to_string()
                };

                let url = self.node.node_resolver.resolve(
                    &specifier,
                    &referrer,
                    NodeModuleKind::Esm,
                    NodeResolutionMode::Execution,
                )?;

                return Ok(url.into_url());
            }

            _ if self.schema_whlist.iter().any(|s| specifier.starts_with(s)) => {
                // Custom schema whitelist import - allow
            }

            _ => {
                return Err(anyhow!(
                    "unrecognized schema for module import: {specifier}"
                ));
            }
        }
        Ok(url)
    }

    pub fn load(
        inner: Rc<RefCell<Self>>,
        module_specifier: &ModuleSpecifier,
        maybe_referrer: Option<&ModuleSpecifier>,
        is_dyn_import: bool,
        requested_module_type: deno_core::RequestedModuleType,
    ) -> deno_core::ModuleLoadResponse {
        let module_specifier = module_specifier.clone();
        let maybe_referrer = maybe_referrer.cloned();

        // Check if the module is in the cache first
        if let Some(cache) = &inner.borrow().cache_provider {
            if let Some(source) = cache.get(&module_specifier) {
                return deno_core::ModuleLoadResponse::Sync(Ok(source));
            }
        }

        // Next check the import provider
        let provider_result = inner.borrow_mut().import_provider.as_mut().and_then(|p| {
            p.import(
                &module_specifier,
                maybe_referrer.as_ref(),
                is_dyn_import,
                requested_module_type,
            )
        });
        if let Some(result) = provider_result {
            return ModuleLoadResponse::Async(
                async move {
                    Self::handle_load(inner, module_specifier, |_, _| async move { result }).await
                }
                .boxed_local(),
            );
        }

        // We check permissions next
        match module_specifier.scheme() {
            // Remote fetch imports
            #[cfg(feature = "url_import")]
            "https" | "http" => ModuleLoadResponse::Async(
                async move { Self::handle_load(inner, module_specifier, Self::load_remote).await }
                    .boxed_local(),
            ),

            // FS imports
            "file" => ModuleLoadResponse::Async(
                async move { Self::handle_load(inner, module_specifier, Self::load_file).await }
                    .boxed_local(),
            ),

            // Default deny-all
            _ => ModuleLoadResponse::Sync(Err(anyhow!(
                "{} imports are not allowed here: {}",
                module_specifier.scheme(),
                module_specifier.as_str()
            ))),
        }
    }

    #[allow(unused_variables)]
    async fn load_file(
        inner: Rc<RefCell<Self>>,
        module_specifier: ModuleSpecifier,
    ) -> Result<String, Error> {
        let path = module_specifier
            .to_file_path()
            .map_err(|()| anyhow!("`{module_specifier}` is not a valid file URL."))?;
        let content = tokio::fs::read_to_string(path).await?;

        #[cfg(feature = "node_experimental")]
        {
            let is_npm = inner
                .borrow()
                .node
                .rusty_resolver
                .in_npm_package(&module_specifier);
            if is_npm {
                let translator = inner.borrow().node.code_translator.clone();
                let _source = translator
                    .translate_cjs_to_esm(
                        &module_specifier,
                        Some(std::borrow::Cow::Borrowed(&content)),
                    )
                    .await?
                    .into_owned();
                // TODO : This breaks some modules
                //   return Ok(source);
            }
        }

        Ok(content)
    }

    #[cfg(feature = "url_import")]
    async fn load_remote(
        _: Rc<RefCell<Self>>,
        module_specifier: ModuleSpecifier,
    ) -> Result<String, Error> {
        let response = reqwest::get(module_specifier).await?;
        Ok(response.text().await?)
    }

    /// Loads a module's source code from the cache or from the provided handler
    async fn handle_load<F, Fut>(
        inner: Rc<RefCell<Self>>,
        module_specifier: ModuleSpecifier,
        handler: F,
    ) -> Result<ModuleSource, deno_core::error::AnyError>
    where
        F: FnOnce(Rc<RefCell<Self>>, ModuleSpecifier) -> Fut,
        Fut: std::future::Future<Output = Result<String, deno_core::error::AnyError>>,
    {
        // Check if the module is in the cache first
        if let Some(Some(source)) = inner
            .borrow()
            .cache_provider
            .as_ref()
            .map(|p| p.get(&module_specifier))
        {
            return Ok(source);
        }

        //
        // Not in the cache, load the module from the handler
        //

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
        let code = handler(inner.clone(), module_specifier.clone()).await?;
        let (tcode, source_map) = transpile(&module_specifier, &code)?;

        // Create the module source
        let mut source = ModuleSource::new(
            module_type,
            ModuleSourceCode::String(tcode.into()),
            &module_specifier,
            None,
        );

        // Add the source to our source cache
        inner.borrow_mut().add_source_map(
            module_specifier.as_str(),
            code,
            source_map.map(|s| s.to_vec()),
        );

        // Cache the source if a cache provider is available
        // Could speed up loads on some future runtime
        if let Some(p) = &mut inner.borrow_mut().cache_provider {
            p.set(&module_specifier, source.clone(&module_specifier));
        }

        // Run import provider post-processing
        if let Some(import_provider) = &mut inner.borrow_mut().import_provider {
            source = import_provider.post_process(&module_specifier, source)?;
        }

        Ok(source)
    }

    /// Returns a reference to a file in the source map cache
    pub fn get_source_map(&self, filename: &str) -> Option<&(String, Option<Vec<u8>>)> {
        self.source_map_cache.get(filename)
    }

    /// Adds a source map to the cache
    pub fn add_source_map(&mut self, filename: &str, source: String, source_map: Option<Vec<u8>>) {
        self.source_map_cache
            .insert(filename.to_string(), (source, source_map));
    }
}
