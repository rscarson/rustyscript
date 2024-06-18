use std::rc::Rc;

use deno_core::{
    anyhow::{self, Ok}, futures::FutureExt, FastString, ModuleLoadResponse, ModuleSource, ModuleSourceCode, ModuleType, ResolutionKind
};
use rustyscript::RuntimeOptions;

// Simply wrap the default module loader to add custom behavior
struct ExampleModuleLoader {
    default_loader: rustyscript::RustyLoader,
}
impl deno_core::ModuleLoader for ExampleModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        kind: ResolutionKind,
    ) -> Result<deno_core::ModuleSpecifier, anyhow::Error> {
        // Default resolution behavior
        self.default_loader.resolve(specifier, referrer, kind)
    }

    fn load(
        &self,
        module_specifier: &deno_core::ModuleSpecifier,
        maybe_referrer: Option<&deno_core::ModuleSpecifier>,
        is_dynamic: bool,
        module_type: deno_core::RequestedModuleType,
    ) -> ModuleLoadResponse {
        // Hard-coded javascript code, only accessible through the `example` scheme
        let code = FastString::from(
            "
            export function add(a, b) {
                return a + b
            }
            "
            .to_string(),
        );
        let specifier = module_specifier.clone();
        let scheme = specifier.scheme();
        match scheme {
            // For this example scheme, the same code is imported regardless of the rest of the specifier
            "example" => ModuleLoadResponse::Async(
                async move {
                    Ok(ModuleSource::new(
                        ModuleType::JavaScript,
                        ModuleSourceCode::String(code),
                        &specifier,
                        None,
                    ))
                }
                .boxed_local(),
            ),
            // All other specifiers fall back to the default loader
            _ => {
                return self.default_loader.load(
                    module_specifier,
                    maybe_referrer,
                    is_dynamic,
                    module_type,
                )
            }
        }
    }
}

fn main() -> Result<(), anyhow::Error> {
    let options = RuntimeOptions {
        module_loader: Some(Rc::new(ExampleModuleLoader {
            default_loader: rustyscript::RustyLoader::new(None),
        })),
        ..Default::default()
    };

    let mut runtime = rustyscript::Runtime::new(options)?;

    let module = rustyscript::Module::new(
        "example.js",
        "
        import { add } from 'example://any_specifier'
        // Use the imported function
        export function test() {
            console.log(add(1, 2))
        }
        ",
    );
    
    let handle = runtime.load_module(&module)?;
    
    runtime.call_function(Some( &handle ), "test", vec![].as_slice())?;

    Ok(())
}
