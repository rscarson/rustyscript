use deno_ast::{MediaType, ModuleSpecifier};
use deno_fs::FileSystem;
use deno_node::{
    DenoFsNodeResolverEnv, NodeExtInitServices, NodeRequireLoader, NodeResolver,
    PackageJsonResolver,
};
use deno_resolver::{
    fs::{DenoResolverFs, DirEntry},
    npm::{ByonmNpmResolver, ByonmNpmResolverCreateOptions},
};
use deno_runtime::ops::process::NpmProcessStateProvider;
use deno_semver::package::PackageReq;
use node_resolver::{
    errors::{ClosestPkgJsonError, PackageFolderResolveErrorKind, PackageNotFoundError},
    InNpmPackageChecker, NpmPackageFolderResolver,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, RwLock},
};

use super::cjs_translator::{NodeCodeTranslator, RustyCjsCodeAnalyzer};

const NODE_MODULES_DIR: &str = "node_modules";

/// Package resolver for the `deno_node` extension
#[derive(Debug)]
pub struct RustyResolver {
    fs: Arc<dyn FileSystem + Send + Sync>,
    byonm: ByonmNpmResolver<ResolverFs, DenoFsNodeResolverEnv>,
    pjson: Arc<PackageJsonResolver>,
    require_loader: RequireLoader,
    root_node_modules_dir: Option<PathBuf>,

    known: RwLock<HashMap<ModuleSpecifier, bool>>,
}
impl Default for RustyResolver {
    fn default() -> Self {
        Self::new(None, Arc::new(deno_fs::RealFs))
    }
}
impl RustyResolver {
    /// Create a new resolver with the given base directory and filesystem
    pub fn new(base_dir: Option<PathBuf>, fs: Arc<dyn FileSystem + Send + Sync>) -> Self {
        let mut base = base_dir;
        if base.is_none() {
            base = std::env::current_dir().ok();
        }

        let root_node_modules_dir = base.map(|mut p| {
            p.push(NODE_MODULES_DIR);
            p
        });

        let pjson = Arc::new(PackageJsonResolver::new(Self::fs_env(fs.clone())));

        let require_loader = RequireLoader(fs.clone());

        let options = ByonmNpmResolverCreateOptions {
            fs: ResolverFs(fs.clone()),
            root_node_modules_dir: root_node_modules_dir.clone(),
            pkg_json_resolver: pjson.clone(),
        };
        let byonm = ByonmNpmResolver::new(options);

        Self {
            fs,
            byonm,
            pjson,
            require_loader,
            root_node_modules_dir,

            known: RwLock::new(HashMap::new()),
        }
    }

    /// Returns a structure capable of translating CJS to ESM
    #[must_use]
    pub fn code_translator(
        self: &Arc<Self>,
        node_resolver: Arc<NodeResolver>,
    ) -> NodeCodeTranslator {
        let cjs = RustyCjsCodeAnalyzer::new(self.filesystem(), self.clone());
        NodeCodeTranslator::new(
            cjs,
            Self::fs_env(self.filesystem()),
            self.clone(),
            node_resolver,
            self.clone(),
            self.pjson.clone(),
        )
    }

    /// Returns a node resolver for the resolver
    #[must_use]
    pub fn node_resolver(self: &Arc<Self>) -> NodeResolver {
        NodeResolver::new(
            Self::fs_env(self.filesystem()),
            self.clone(),
            self.clone(),
            self.pjson.clone(),
        )
    }

    /// Returns the package.json resolver used by the resolver
    pub fn package_json_resolver(&self) -> Arc<PackageJsonResolver> {
        self.pjson.clone()
    }

    /// Resolves an importalias for a given specifier
    pub fn resolve_alias(&self, specifier: &str, referrer: &ModuleSpecifier) -> Option<String> {
        let package = self
            .package_json_resolver()
            .get_closest_package_json(referrer)
            .ok()??;
        let imports = package.imports.as_ref()?;
        let alias = imports.get(specifier)?;

        if let Some(obj) = alias.as_object() {
            if let Some(node) = obj.get("node") {
                if let Some(alias) = node.as_str() {
                    return Some(alias.to_string());
                }
            }
        } else if let Some(str) = alias.as_str() {
            return Some(str.to_string());
        }

        None
    }

    fn get_known_is_cjs(&self, specifier: &ModuleSpecifier) -> Option<bool> {
        self.known
            .read()
            .ok()
            .and_then(|k| k.get(specifier).copied())
    }

    fn set_is_cjs(&self, specifier: &ModuleSpecifier, value: bool) {
        if let Ok(mut known) = self.known.write() {
            known.insert(specifier.clone(), value);
        }
    }

    fn check_based_on_pkg_json(
        &self,
        specifier: &ModuleSpecifier,
    ) -> Result<bool, ClosestPkgJsonError> {
        if self.in_npm_package(specifier) {
            if let Some(pkg_json) = self.pjson.get_closest_package_json(specifier)? {
                let is_file_location_cjs = pkg_json.typ != "module";
                Ok(is_file_location_cjs)
            } else {
                Ok(true)
            }
        } else if let Some(pkg_json) = self.pjson.get_closest_package_json(specifier)? {
            let is_cjs_type = pkg_json.typ == "commonjs";
            Ok(is_cjs_type)
        } else {
            Ok(false)
        }
    }

    /// Returns true if the given specifier is a `CommonJS` module
    /// based on the package.json of the module or the specifier itself
    ///
    /// Used to transpile `CommonJS` modules to ES modules
    pub fn is_cjs(
        &self,
        specifier: &ModuleSpecifier,
        media_type: MediaType,
        is_script: bool,
    ) -> bool {
        if specifier.scheme() != "file" {
            return false;
        }

        match media_type {
            MediaType::Wasm
            | MediaType::Json
            | MediaType::Mts
            | MediaType::Mjs
            | MediaType::Dmts => false,

            MediaType::Cjs | MediaType::Cts | MediaType::Dcts => true,

            MediaType::Dts => {
                // dts files are always determined based on the package.json because
                // they contain imports/exports even when considered CJS
                if let Some(value) = self.get_known_is_cjs(specifier) {
                    value
                } else {
                    let value = self.check_based_on_pkg_json(specifier).ok();
                    if let Some(value) = value {
                        self.set_is_cjs(specifier, value);
                    }
                    value.unwrap_or(false)
                }
            }

            MediaType::JavaScript
            | MediaType::Jsx
            | MediaType::TypeScript
            | MediaType::Tsx
            | MediaType::Css
            | MediaType::SourceMap
            | MediaType::Unknown => {
                if let Some(value) = self.get_known_is_cjs(specifier) {
                    if value && !is_script {
                        // we now know this is actually esm
                        self.set_is_cjs(specifier, false);
                        false
                    } else {
                        value
                    }
                } else if !is_script {
                    // we now know this is esm
                    self.set_is_cjs(specifier, false);
                    false
                } else {
                    let value = self.check_based_on_pkg_json(specifier).ok();
                    if let Some(value) = value {
                        self.set_is_cjs(specifier, value);
                    }
                    value.unwrap_or(false)
                }
            }
        }
    }

    /// Returns true if a `node_modules` directory exists in the base directory
    /// and is a directory.
    #[must_use]
    pub fn has_node_modules_dir(&self) -> bool {
        self.root_node_modules_dir
            .as_ref()
            .is_some_and(|d| self.fs.exists_sync(d) && self.fs.is_dir_sync(d))
    }

    /// Returns the filesystem implementation used by the resolver
    #[must_use]
    pub fn filesystem(&self) -> Arc<dyn FileSystem + Send + Sync> {
        self.fs.clone()
    }

    /// Initializes the services required by the resolver
    #[must_use]
    pub fn init_services(self: &Arc<Self>) -> NodeExtInitServices {
        NodeExtInitServices {
            node_require_loader: Rc::new(self.require_loader.clone()),
            pkg_json_resolver: self.pjson.clone(),
            node_resolver: Arc::new(self.node_resolver()),
            npm_resolver: self.clone(),
        }
    }

    fn fs_env(fs: Arc<dyn FileSystem + Send + Sync>) -> DenoFsNodeResolverEnv {
        DenoFsNodeResolverEnv::new(fs)
    }
}

impl InNpmPackageChecker for RustyResolver {
    fn in_npm_package(&self, specifier: &reqwest::Url) -> bool {
        let is_file = specifier.scheme() == "file";

        let path = specifier.path().to_ascii_lowercase();
        let in_node_modules = path.contains("/node_modules/");
        let is_polyfill = path.contains("/node:");

        is_file && (in_node_modules || is_polyfill)
    }
}

impl NpmPackageFolderResolver for RustyResolver {
    fn resolve_package_folder_from_package(
        &self,
        specifier: &str,
        referrer: &reqwest::Url,
    ) -> Result<PathBuf, node_resolver::errors::PackageFolderResolveError> {
        let request = PackageReq::from_str(specifier).map_err(|_| {
            let e = Box::new(PackageFolderResolveErrorKind::PackageNotFound(
                PackageNotFoundError {
                    package_name: specifier.to_string(),
                    referrer: referrer.clone(),
                    referrer_extra: None,
                },
            ));
            node_resolver::errors::PackageFolderResolveError(e)
        })?;

        let p = self
            .byonm
            .resolve_pkg_folder_from_deno_module_req(&request, referrer);
        match p {
            Ok(p) => Ok(p),
            Err(_) => self
                .byonm
                .resolve_package_folder_from_package(specifier, referrer),
        }
    }
}

/// State provided to the process via an environment variable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NpmProcessState {
    pub kind: NpmProcessStateKind,
    pub local_node_modules_path: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NpmProcessStateKind {
    Byonm,
}
impl NpmProcessStateProvider for RustyResolver {
    fn get_npm_process_state(&self) -> String {
        let modules_path = self
            .root_node_modules_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string());
        let state = NpmProcessState {
            kind: NpmProcessStateKind::Byonm,
            local_node_modules_path: modules_path,
        };
        deno_core::serde_json::to_string(&state).unwrap_or_default()
    }
}

#[derive(Debug)]
struct RequireLoader(Arc<dyn FileSystem + Send + Sync>);
impl NodeRequireLoader for RequireLoader {
    fn load_text_file_lossy(
        &self,
        path: &Path,
    ) -> Result<Cow<'static, str>, deno_core::error::AnyError> {
        let media_type = MediaType::from_path(path);
        let text = self.0.read_text_file_lossy_sync(path, None)?;
        Ok(text)
    }

    fn ensure_read_permission<'a>(
        &self,
        permissions: &mut dyn deno_node::NodePermissions,
        path: &'a Path,
    ) -> Result<std::borrow::Cow<'a, Path>, deno_core::error::AnyError> {
        let is_in_node_modules = path
            .components()
            .all(|c| c.as_os_str().to_ascii_lowercase() != NODE_MODULES_DIR);
        if is_in_node_modules {
            permissions.check_read_path(path).map_err(Into::into)
        } else {
            Ok(Cow::Borrowed(path))
        }
    }

    fn is_maybe_cjs(&self, specifier: &reqwest::Url) -> Result<bool, ClosestPkgJsonError> {
        if specifier.scheme() != "file" {
            return Ok(false);
        }

        match MediaType::from_specifier(specifier) {
            MediaType::Wasm
            | MediaType::Json
            | MediaType::Mts
            | MediaType::Mjs
            | MediaType::Dmts => Ok(false),

            _ => Ok(true),
        }
    }
}
impl Clone for RequireLoader {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug, Clone)]
struct ResolverFs(Arc<dyn FileSystem + Send + Sync>);
impl DenoResolverFs for ResolverFs {
    fn exists_sync(&self, path: &Path) -> bool {
        self.0.exists_sync(path)
    }

    fn read_to_string_lossy(&self, path: &Path) -> std::io::Result<Cow<'static, str>> {
        self.0
            .read_text_file_lossy_sync(path, None)
            .map_err(deno_io::fs::FsError::into_io_error)
    }

    fn realpath_sync(&self, path: &Path) -> std::io::Result<PathBuf> {
        self.0
            .realpath_sync(path)
            .map_err(deno_io::fs::FsError::into_io_error)
    }

    fn is_dir_sync(&self, path: &Path) -> bool {
        self.0.is_dir_sync(path)
    }

    fn read_dir_sync(&self, dir_path: &Path) -> std::io::Result<Vec<DirEntry>> {
        self.0
            .read_dir_sync(dir_path)
            .map(|entries| {
                entries
                    .into_iter()
                    .map(|e| DirEntry {
                        name: e.name,
                        is_file: e.is_file,
                        is_directory: e.is_directory,
                    })
                    .collect::<Vec<_>>()
            })
            .map_err(deno_io::fs::FsError::into_io_error)
    }
}
