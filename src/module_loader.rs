use crate::transpiler;
use deno_core::{
    anyhow,
    anyhow::anyhow,
    futures::{self, FutureExt},
    ModuleLoader, ModuleSource, ModuleSpecifier, ModuleType,
};
use std::{collections::HashSet, sync::Mutex};

#[cfg(feature = "url_import")]
async fn load_from_url(
    module_specifier: ModuleSpecifier,
) -> Result<ModuleSource, deno_core::error::AnyError> {
    let module_type = if module_specifier.path().ends_with(".json") {
        ModuleType::Json
    } else {
        ModuleType::JavaScript
    };

    let response = reqwest::get(module_specifier.as_str()).await?;
    let code = response.text().await?;
    let code = transpiler::transpile(&module_specifier, &code)?;

    Ok(ModuleSource::new(
        module_type,
        code.into(),
        &module_specifier,
    ))
}

async fn load_from_file(
    module_specifier: ModuleSpecifier,
) -> Result<ModuleSource, deno_core::error::AnyError> {
    let module_type = if module_specifier.path().ends_with(".json") {
        ModuleType::Json
    } else {
        ModuleType::JavaScript
    };

    let path = module_specifier.to_file_path().map_err(|_| {
        anyhow!("Provided module specifier \"{module_specifier}\" is not a file URL.")
    })?;
    let code = std::fs::read_to_string(path)?;
    let code = transpiler::transpile(&module_specifier, &code)?;

    Ok(ModuleSource::new(
        module_type,
        code.into(),
        &module_specifier,
    ))
}

pub struct RustyLoader {
    fs_whlist: Mutex<HashSet<String>>,
}
#[allow(unreachable_code)]
impl ModuleLoader for RustyLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: deno_core::ResolutionKind,
    ) -> Result<ModuleSpecifier, anyhow::Error> {
        let url = deno_core::resolve_import(specifier, &referrer)?;
        if referrer == "." {
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

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
    ) -> std::pin::Pin<Box<deno_core::ModuleSourceFuture>> {
        // We check permissions first
        match module_specifier.scheme() {
            // Remote fetch imports
            #[cfg(feature = "url_import")]
            "https" | "http" => load_from_url(module_specifier.clone()).boxed_local(),

            // FS imports
            "file" => load_from_file(module_specifier.clone()).boxed_local(),

            _ => futures::future::ready(Err(anyhow!(
                "{} imports are not allowed here: {}",
                module_specifier.scheme(),
                module_specifier.as_str()
            )))
            .boxed_local(),
        }
    }
}

#[allow(dead_code)]
impl RustyLoader {
    pub fn new() -> Self {
        Self {
            fs_whlist: Mutex::new(Default::default()),
        }
    }

    pub fn whitelist_add(&self, specifier: &str) {
        if let Ok(mut whitelist) = self.fs_whlist.lock() {
            whitelist.insert(specifier.to_string());
        }
    }

    pub fn whitelist_has(&self, specifier: &str) -> bool {
        if let Ok(whitelist) = self.fs_whlist.lock() {
            whitelist.contains(specifier)
        } else {
            false
        }
    }
}
