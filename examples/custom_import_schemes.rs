use deno_core::anyhow;
use rustyscript::{DefaultImporter, ImportProvider, Module, Runtime, RuntimeOptions};

struct CustomImporter {
    fallback_importer: DefaultImporter,
}
impl CustomImporter {
    pub fn new() -> Self {
        Self {
            fallback_importer: DefaultImporter,
        }
    }
}
impl ImportProvider for CustomImporter {
    fn import(
        &self,
        specifier: deno_core::ModuleSpecifier,
    ) -> Result<String, anyhow::Error> {
        // Define custom import behavior, depending on the specifier
        match specifier.scheme() {
            // Import from schemes that aren't `file` or `https`
            "example" => {
                Ok("
                    export const test = 'Only those who master the art of the custom import scheme can print this string...'; 
                ".to_string())
            }
            // Fall back to the default import behavior (or the behavior of another import provider)
            _ => self.fallback_importer.import(specifier),
        }
    }
}

fn main() -> Result<(), anyhow::Error> {
    let options = RuntimeOptions {
        import_provider: Some(Box::new(CustomImporter::new())),
        ..Default::default()
    };
    let mut runtime = Runtime::new(options)?;
    let module = Module::new(
        "example.js",
        "
        import { test } from 'example://any-specifier';
        console.log(test);
        "
    );
    
    runtime.load_module(&module)?;

    Ok(())    
}