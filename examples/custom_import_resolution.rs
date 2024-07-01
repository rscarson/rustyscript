use deno_core::{ModuleSpecifier, ResolutionKind};
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
        specifier: &ModuleSpecifier,
        _referrer: &str,
        _kind: ResolutionKind,
    ) -> std::option::Option<Result<ModuleSpecifier, deno_core::anyhow::Error>> {
        if specifier.as_str() == "example:secret_special_specifier" {
            // Substitute another URL in certain situations...
            Some(ModuleSpecifier::parse(&self.replacement_specifier).map_err(|e| e.into()))
        } else {
            // Or fall back to the default resolve behavior
            None
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
        import { assertEquals } from 'example:secret_special_specifier'
        
        assertEquals(1, 1)
        ",
    );

    runtime.load_module(&module).expect("Could not load module");
}
