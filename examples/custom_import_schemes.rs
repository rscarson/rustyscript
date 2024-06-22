use std::pin::Pin;

use deno_core::{anyhow, futures::FutureExt};
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
    ) -> Pin<Box<dyn std::future::Future<
        Output = Result<std::string::String, deno_core::anyhow::Error>
    >>> {
        // Define custom import behavior, depending on the specifier
        match specifier.scheme() {
            // Import from schemes that aren't `file` or `https`
            "example" => async move {
                Ok("
                    export const test = 'Only those who master the art of the custom import scheme can print this string...'; 
                ".to_string())
            }.boxed_local(),
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
        ",
    );

    runtime.load_module(&module)?;

    Ok(())
}
