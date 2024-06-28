use deno_core::{anyhow::Error, ModuleSpecifier, ResolutionKind};
use rustyscript::{ImportProvider, Runtime};

struct MyImportProvider {
    replacement_specifier: String,
}
impl MyImportProvider {
    fn new(str: &str) -> Self {
        Self {
            replacement_specifier: String::from(str),
        }
    }
}
impl ImportProvider for MyImportProvider {
    fn resolve(
        &mut self,
        specifier: &str,
        referrer: &str,
        _kind: ResolutionKind,
    ) -> std::result::Result<ModuleSpecifier, Error> {
        if specifier == "secret_special_specifier" {
            // Substitute another URL in certain situations...
            Ok(ModuleSpecifier::parse(&self.replacement_specifier)?)
        } else {
            // Or fall back to deno's import behavior (also used by rustyscript)
            Ok(deno_core::resolve_import(specifier, referrer)?)
        }
    }
}
fn main() {
    let options = rustyscript::RuntimeOptions {
        import_provider: Some(Box::new(MyImportProvider::new("https://deno.land/std@0.224.0/assert/mod.ts"))),
        ..Default::default()
    };
    let mut runtime = Runtime::new(options).expect("Could not create runtime");

    let module = rustyscript::Module::new(
        "custom_imports.js",
        "
        import { assertEquals } from 'secret_special_specifier'
        
        assertEquals(1, 1)
        ",
    );

    runtime.load_module(&module).expect("Could not load module");
}
