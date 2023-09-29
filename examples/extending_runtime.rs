///
/// This example demonstrates extending Runtime to inline your own extensions and modules
/// as well as enforce values for the Runtime's options
///
/// This example creates a runtime which will timeout after 0.5s, imports an exension,
/// And ensures that a preset module is always available for import.
///
/// Extensions like the one being used (see examples/ext/example_extension.rs)
/// allow you to call rust code from within JS
///
/// Extensions consist of a set of #[op2] functions, an extension! macro,
/// and one or more optional JS modules.
///
use js_playground::{serde_json, Error, ModuleHandle, Runtime, RuntimeOptions, Script};
use std::time::Duration;

mod ext;
use ext::example_extension;

/// A runtime which will timeout after 0.5s, imports an exension,
/// And ensures that a preset module is always available for import.
pub struct MyRuntime(Runtime);
impl MyRuntime {
    /// Create a new instance of the runtime
    pub fn new() -> Result<Self, Error> {
        let mut runtime = Self(Runtime::new(RuntimeOptions {
            extensions: vec![example_extension::example_extension::init_ops_and_esm()],
            timeout: Some(Duration::from_millis(500)),
            ..Default::default()
        })?);

        runtime.reset();
        Ok(runtime)
    }

    /// Calls a JavaScript function within the Deno runtime by its name and deserializes its return value.
    ///
    /// # Arguments
    /// * `name` - A string representing the name of the JavaScript function to call.
    pub fn call_function<T>(
        &mut self,
        module_context: &ModuleHandle,
        name: &str,
        args: &[serde_json::Value],
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        self.0.call_function(module_context, name, args)
    }

    /// Get a value from a runtime instance
    ///
    /// # Arguments
    /// * `name` - A string representing the name of the value to find
    pub fn get_value<T>(&mut self, module_context: &ModuleHandle, name: &str) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        self.0.get_value(module_context, name)
    }

    /// Executes the given script, and returns a handle allowing you to extract values
    /// And call functions
    ///
    /// # Arguments
    /// * `module` - A `Script` object containing the module's filename and contents.
    pub fn load_module(&mut self, module: &Script) -> Result<ModuleHandle, Error> {
        self.0.load_module(module)
    }

    /// Reset the runtime
    /// This clears any side-effects in global, and unloads any running modules
    pub fn reset(&mut self) {
        let important_module = Script::new(
            "my_module.js",
            "
            export function importantFunction() {
                return 42;
            }
        ",
        );

        self.0.reset();
        self.load_module(&important_module)
            .expect("Could not load default module!");
    }
}

fn main() {
    let script = Script::new(
        "test.js",
        "
        import { importantFunction } from './my_module.js';
        export const result = importantFunction();
        example_ext.add(2, 5);
        ",
    );

    let mut runtime = MyRuntime::new().expect("Could not create the runtime");
    let module_handle = runtime
        .load_module(&script)
        .expect("Error loading the module");
    assert_eq!(
        42,
        runtime
            .get_value::<i64>(&module_handle, "result")
            .expect("Could not get value from the module")
    );
}
