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
use js_playground::{
    module, serde_json, Error, Module, ModuleHandle, Runtime, RuntimeOptions, StaticModule,
};
use std::time::Duration;

mod ext;
use ext::example_extension;

// A module that will always be loaded into the custom runtime
const MY_MODULE: StaticModule = module!(
    "my_module.js",
    "export function importantFunction() {
        return 42;
    }"
);

/// A runtime which will timeout after 0.5s, imports an exension,
/// And ensures that a preset module is always available for import.
pub struct MyRuntime(Runtime);
impl MyRuntime {
    /// Create a new instance of the runtime
    pub fn new() -> Result<Self, Error> {
        let mut runtime = Self(Runtime::new(RuntimeOptions {
            extensions: vec![example_extension::example_extension::init_ops_and_esm()],
            timeout: Duration::from_millis(500),
            ..Default::default()
        })?);

        runtime.reset();
        Ok(runtime)
    }

    /// Calls a javascript function within the Deno runtime by its name and deserializes its return value.
    ///
    /// # Arguments
    /// * `name` - A string representing the name of the javascript function to call.
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
        T: serde::de::DeserializeOwned,
    {
        self.0.get_value(module_context, name)
    }

    /// Executes the given module, and returns a handle allowing you to extract values
    /// And call functions
    ///
    /// # Arguments
    /// * `module` - A `Module` object containing the module's filename and contents.
    pub fn load_module(&mut self, module: &Module) -> Result<ModuleHandle, Error> {
        self.0.load_module(module)
    }

    /// Reset the runtime
    /// This clears any side-effects in global, and unloads any running modules
    pub fn reset(&mut self) {
        let important_module = Module::new(
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
    let mut runtime = MyRuntime::new().expect("Could not create the runtime");
    let module_handle = runtime
        .load_module(&MY_MODULE.to_module())
        .expect("Error loading the module");
    assert_eq!(
        42,
        runtime
            .get_value::<i64>(&module_handle, "result")
            .expect("Could not get value from the module")
    );
}
