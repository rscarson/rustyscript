use deno_core::ModuleSpecifier;
use rustyscript::{ImportProvider, Runtime};

struct MyImportProvider {
    example_code: String,
}
impl MyImportProvider {
    fn new(str: &str) -> Self {
        Self {
            example_code: String::from(str),
        }
    }
}
impl ImportProvider for MyImportProvider {
    fn resolve(
            &mut self,
            specifier: &ModuleSpecifier,
            _referrer: &str,
            _kind: deno_core::ResolutionKind,
        ) -> Option<Result<ModuleSpecifier, deno_core::anyhow::Error>> {
        // Provide a resolution only if our custom URL scheme is used
        if specifier.scheme() == "examplescheme" {
            Some(Ok(specifier.clone()))
        } else {
            None
        }
    }
    fn import(
        &mut self,
        specifier: &ModuleSpecifier,
        _referrer: &Option<ModuleSpecifier>,
        _is_dyn_import: bool,
        _requested_module_type: deno_core::RequestedModuleType,
    ) -> Option<Result<String, deno_core::anyhow::Error>> {
        // Load module from example_code for this one scheme, but otherwise return None
        if specifier.scheme() == "examplescheme" {
            Some(Ok(self.example_code.clone()))
        } else {
            None
        }
    }
}
fn main() {
    let options = rustyscript::RuntimeOptions {
        import_provider: Some(Box::new(MyImportProvider::new(
            "
            export function return_number(n) {
                return n
            }
        ",
        ))),
        ..Default::default()
    };
    let mut runtime = Runtime::new(options).expect("Could not create runtime");

    let module = rustyscript::Module::new(
        "custom_imports.js",
        "
        import { assertEquals } from 'https://deno.land/std@0.224.0/assert/mod.ts'
        import { return_number } from 'examplescheme://any_specifier'
        
        assertEquals(return_number(1), 1)
        ",
    );

    runtime.load_module(&module).expect("Could not load module");
}
