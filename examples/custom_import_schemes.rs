use deno_core::{anyhow::Error, ModuleSpecifier};
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
    fn import(
            &mut self,
            specifier: &ModuleSpecifier,
            _referrer: &Option<ModuleSpecifier>,
            _is_dyn_import: bool,
            _requested_module_type: deno_core::RequestedModuleType,
        ) -> Result<String, deno_core::anyhow::Error> {
        if specifier.scheme() == "examplescheme" {
            // Load code from anywhere 
            Ok(self.example_code.clone())
        } else {
            // Make sure to handle unrecognized schemes as well
            Err(Error::msg(format!("unrecognized scheme for module import: {specifier}")))
        }
    }
}
fn main() {
    let options = rustyscript::RuntimeOptions {
        import_provider: Some(Box::new(MyImportProvider::new("
            export function return_number(n) {
                return n
            }
        "))),
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
