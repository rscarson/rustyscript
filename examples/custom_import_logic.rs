//!
//! In this example I will demonstrate how to override the default import logic of the runtime by implementing a custom
//! import provider.
//!
//! In this case, I will allowing for two new import schemes:
//! - `static:`: This scheme will allow for static modules to be imported by their specifier
//! - `redirect:`: This scheme will allow for modules to be redirected to a different specifier
//!
use deno_core::{anyhow::anyhow, ModuleSpecifier};
use rustyscript::{module_loader::ImportProvider, Module, Runtime, RuntimeOptions};
use std::collections::HashMap;

/// A custom import provider that allows for static modules and redirects
#[derive(Default)]
struct MyImportProvider {
    static_modules: HashMap<String, String>,
    redirects: HashMap<String, ModuleSpecifier>,
}
impl MyImportProvider {
    //
    // The schemes we will be using
    const STATIC_SCHEME: &'static str = "static";
    const REDIRECT_SCHEME: &'static str = "redirect";

    /// Add a static module to the provider
    fn add_static_module(&mut self, specifier: &str, source: &str) {
        self.static_modules
            .insert(specifier.to_string(), source.to_string());
    }

    /// Add a redirect to the provider
    fn add_redirect(&mut self, from: &str, to: &str) -> Result<(), deno_core::error::AnyError> {
        self.redirects
            .insert(from.to_string(), ModuleSpecifier::parse(to)?);
        Ok(())
    }
}

impl ImportProvider for MyImportProvider {
    fn resolve(
        &mut self,
        specifier: &ModuleSpecifier,
        _referrer: &str,
        _kind: deno_core::ResolutionKind,
    ) -> Option<Result<ModuleSpecifier, deno_core::anyhow::Error>> {
        match specifier.scheme() {
            //
            // static:*, use the static module set
            Self::STATIC_SCHEME => {
                if self.static_modules.contains_key(specifier.path()) {
                    // Import is allowed - return the specifier
                    Some(Ok(specifier.clone()))
                } else {
                    // Not found - deny the import
                    Some(Err(anyhow!("Static module not found: {}", specifier)))
                }
            }

            //
            // redirect:*, use the redirect set
            Self::REDIRECT_SCHEME => {
                if let Some(redirected_specifier) = self.redirects.get(specifier.path()) {
                    // Redirected - return the redirected specifier
                    Some(Ok(redirected_specifier.clone()))
                } else {
                    // No redirect, deny the import
                    Some(Err(anyhow!("Module redirect not found: {}", specifier)))
                }
            }

            // Not in scope for us, let the standard loader handle it
            _ => None,
        }
    }

    fn import(
        &mut self,
        specifier: &ModuleSpecifier,
        _referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
        _requested_module_type: deno_core::RequestedModuleType,
    ) -> Option<Result<String, deno_core::anyhow::Error>> {
        match specifier.scheme() {
            //
            // static:*, use the static module set
            Self::STATIC_SCHEME => {
                if let Some(source) = self.static_modules.get(specifier.path()) {
                    // Found, return the source
                    Some(Ok(source.clone()))
                } else {
                    // Not found, deny the import
                    Some(Err(anyhow!("Static module not found: {}", specifier)))
                }
            }

            //
            // Let the standard loader handle redirected specifiers
            _ => None,
        }
    }
}

fn main() -> Result<(), rustyscript::Error> {
    let mut import_provider = MyImportProvider::default();
    import_provider.add_redirect("mod_assert", "https://deno.land/std@0.224.0/assert/mod.ts")?;
    import_provider.add_static_module("my-module", "export const foo = 1");

    let mut runtime = Runtime::new(RuntimeOptions {
        import_provider: Some(Box::new(import_provider)),
        ..Default::default()
    })?;

    let module = Module::new(
        "custom_imports.js",
        "
        import { assertEquals } from 'redirect:mod_assert';
        import { foo } from 'static:my-module';
        
        assertEquals(1, foo)
        ",
    );

    runtime.load_module(&module)?;
    Ok(())
}
