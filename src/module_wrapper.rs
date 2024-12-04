use crate::{js_value::Function, Error, Module, ModuleHandle, Runtime, RuntimeOptions};
use deno_core::{serde_json, v8::GetPropertyNamesArgs};

/// A wrapper type representing a runtime instance loaded with a single module
///
/// Exactly equivalent to [`Runtime::new`] followed by [`Runtime::load_module`]
///
/// Can also be created using the [`crate::import`] function
pub struct ModuleWrapper {
    module_context: ModuleHandle,
    runtime: Runtime,
}

impl ModuleWrapper {
    /// Creates a new `ModuleWrapper` from a given module and runtime options.
    ///
    /// # Arguments
    /// * `module` - A reference to the module to load.
    /// * `options` - The runtime options for the module.
    ///
    /// # Returns
    /// A `Result` containing `Self` on success or an `Error` on failure.
    ///
    /// # Errors
    /// Will return an error if module execution fails
    pub fn new_from_module(module: &Module, options: RuntimeOptions) -> Result<Self, Error> {
        let mut runtime = Runtime::new(options)?;
        let module_context = runtime.load_module(module)?;
        Ok(Self {
            module_context,
            runtime,
        })
    }

    /// Creates a new `ModuleWrapper` from a file path and runtime options.
    ///
    /// # Arguments
    /// * `path` - The path to the module file.
    /// * `options` - The runtime options for the module.
    ///
    /// # Returns
    /// A `Result` containing `Self` on success or an `Error` on failure.
    ///
    /// # Errors
    /// Will return an error if the file cannot be loaded, or if module execution fails
    pub fn new_from_file(path: &str, options: RuntimeOptions) -> Result<Self, Error> {
        let module = Module::load(path)?;
        Self::new_from_module(&module, options)
    }

    /// Returns a reference to the module context.
    #[must_use]
    pub fn get_module_context(&self) -> &ModuleHandle {
        &self.module_context
    }

    /// Returns a mutable reference to the underlying runtime.
    pub fn get_runtime(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    /// Retrieves a value from the module by name and deserializes it.
    ///
    /// See [`Runtime::get_value`]
    ///
    /// # Arguments
    /// * `name` - The name of the value to retrieve.
    ///
    /// # Returns
    /// A `Result` containing the deserialized value of type `T` on success or an `Error` on failure.
    ///
    /// # Errors
    /// Will return an error if the value cannot be found, or deserialized into the given type
    pub fn get<T>(&mut self, name: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.runtime.get_value(Some(&self.module_context), name)
    }

    /// Retrieves a future resolving to a value from the module by name and deserializes it.
    ///
    /// See [`Runtime::get_value_async`]
    ///
    /// # Arguments
    /// * `name` - The name of the value to retrieve.
    ///
    /// # Returns
    /// A `Result` containing the deserialized value of type `T` on success or an `Error` on failure.
    ///
    /// # Errors
    /// Will return an error if the value cannot be found, or deserialized into the given type
    pub async fn get_async<T>(&mut self, name: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.runtime
            .get_value_async(Some(&self.module_context), name)
            .await
    }

    /// Retrieves a value from the module by name and deserializes it.
    ///
    /// Does not await promises or the event loop.
    ///
    /// See [`Runtime::get_value_immediate`]
    ///
    /// # Arguments
    /// * `name` - The name of the value to retrieve.
    ///
    /// # Returns
    /// A `Result` containing the deserialized value of type `T` on success or an `Error` on failure.
    ///
    /// # Errors
    /// Will return an error if the value cannot be found, or deserialized into the given type
    pub fn get_immediate<T>(&mut self, name: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.runtime
            .get_value_immediate(Some(&self.module_context), name)
    }

    /// Checks if a value in the module with the given name is callable as a JavaScript function.
    ///
    /// # Arguments
    /// * `name` - The name of the value to check for callability.
    ///
    /// # Returns
    /// `true` if the value is callable as a JavaScript function, `false` otherwise.
    pub fn is_callable(&mut self, name: &str) -> bool {
        let test = self.get::<Function>(name);
        test.is_ok()
    }

    /// Calls a function in the module with the given name and arguments and deserializes the result.
    ///
    /// See [`Runtime::call_function`]
    ///
    /// # Arguments
    /// * `name` - The name of the function to call.
    /// * `args` - The arguments to pass to the function.
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of type `T` on success or an `Error` on failure.
    ///
    /// # Errors
    /// Will return an error if the function cannot be called, if the function returns an error,
    /// or if the function returns a value that cannot be deserialized into the given type
    pub fn call<T>(&mut self, name: &str, args: &impl serde::ser::Serialize) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.runtime
            .call_function(Some(&self.module_context), name, args)
    }

    /// Calls a function in the module with the given name and arguments and deserializes the result.
    ///
    /// See [`Runtime::call_function_async`]
    ///
    /// # Arguments
    /// * `name` - The name of the function to call.
    /// * `args` - The arguments to pass to the function.
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of type `T` on success or an `Error` on failure.
    ///
    /// # Errors
    /// Will return an error if the function cannot be called, if the function returns an error,
    /// or if the function returns a value that cannot be deserialized into the given type
    pub async fn call_async(
        &mut self,
        name: &str,
        args: &impl serde::ser::Serialize,
    ) -> Result<serde_json::Value, Error> {
        self.runtime
            .call_function_async(Some(&self.module_context), name, args)
            .await
    }

    /// Calls a function in the module with the given name and arguments and deserializes the result.  
    /// Does not await promises or the event loop.
    ///
    /// See [`Runtime::call_function_immediate`]
    ///
    /// # Arguments
    /// * `name` - The name of the function to call.
    /// * `args` - The arguments to pass to the function.
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of type `T` on success or an `Error` on failure.
    ///
    /// # Errors
    /// Will return an error if the function cannot be called, if the function returns an error,
    /// or if the function returns a value that cannot be deserialized into the given type
    pub fn call_immediate(
        &mut self,
        name: &str,
        args: &impl serde::ser::Serialize,
    ) -> Result<serde_json::Value, Error> {
        self.runtime
            .call_function_immediate(Some(&self.module_context), name, args)
    }

    /// Calls a function using the module's runtime that was previously stored as a Function object
    ///
    /// See [`Runtime::call_stored_function`]
    ///
    /// # Arguments
    /// * `function` - The Function to call.
    /// * `args` - The arguments to pass to the function.
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of type `T` on success or an `Error` on failure.
    ///
    /// # Errors
    /// Will return an error if the function cannot be called, if the function returns an error,
    /// or if the function returns a value that cannot be deserialized into the given type
    pub fn call_stored<T>(
        &mut self,
        function: &Function,
        args: &impl serde::ser::Serialize,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.runtime
            .call_stored_function(Some(&self.module_context), function, args)
    }

    /// Calls a function using the module's runtime that was previously stored as a Function object
    ///
    /// See [`Runtime::call_stored_function_async`]
    ///
    /// # Arguments
    /// * `function` - The Function to call.
    /// * `args` - The arguments to pass to the function.
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of type `T` on success or an `Error` on failure.
    ///
    /// # Errors
    /// Will return an error if the function cannot be called, if the function returns an error,
    /// or if the function returns a value that cannot be deserialized into the given type
    pub async fn call_stored_async(
        &mut self,
        function: &Function,
        args: &impl serde::ser::Serialize,
    ) -> Result<serde_json::Value, Error> {
        self.runtime
            .call_stored_function_async(Some(&self.module_context), function, args)
            .await
    }

    /// Calls a function using the module's runtime that was previously stored as a Function object
    ///
    /// Does not await promises or the event loop.
    ///
    /// See [`Runtime::call_stored_function_immediate`]
    ///
    /// # Arguments
    /// * `function` - The Function to call.
    /// * `args` - The arguments to pass to the function.
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of type `T` on success or an `Error` on failure.
    ///
    /// # Errors
    /// Will return an error if the function cannot be called, if the function returns an error,
    /// or if the function returns a value that cannot be deserialized into the given type
    pub fn call_stored_immediate(
        &mut self,
        function: &Function,
        args: &impl serde::ser::Serialize,
    ) -> Result<serde_json::Value, Error> {
        self.runtime
            .call_stored_function_immediate(Some(&self.module_context), function, args)
    }

    /// Retrieves the names of the module's exports.  
    /// (Keys that are not valid UTF-8, may not work as intended due to encoding issues)
    ///
    /// # Returns
    /// A `Vec` of `String` containing the names of the keys.
    pub fn keys(&mut self) -> Vec<String> {
        let mut keys: Vec<String> = Vec::new();
        if let Ok(namespace) = self
            .runtime
            .deno_runtime()
            .get_module_namespace(self.module_context.id())
        {
            let mut scope = self.runtime.deno_runtime().handle_scope();
            let global = namespace.open(&mut scope);
            if let Some(keys_obj) =
                global.get_property_names(&mut scope, GetPropertyNamesArgs::default())
            {
                for i in 0..keys_obj.length() {
                    if let Ok(key_index) = deno_core::serde_v8::to_v8(&mut scope, i) {
                        if let Some(key_name_v8) = keys_obj.get(&mut scope, key_index) {
                            let name = key_name_v8.to_rust_string_lossy(&mut scope);
                            keys.push(name);
                        }
                    }
                }
            }
        }

        keys
    }
}

#[cfg(test)]
mod test_runtime {
    use super::*;
    use crate::json_args;

    #[test]
    fn test_call() {
        let module = Module::new(
            "test.js",
            "
            console.log('test');
            export const value = 3;
            export function func() { return 4; }
        ",
        );

        let mut module = ModuleWrapper::new_from_module(&module, RuntimeOptions::default())
            .expect("Could not create wrapper");
        let value: usize = module
            .call("func", json_args!())
            .expect("Could not call function");
        assert_eq!(4, value);
    }

    #[test]
    fn test_get() {
        let module = Module::new(
            "test.js",
            "
            export const value = 3;
            export function func() { return 4; }
        ",
        );

        let mut module = ModuleWrapper::new_from_module(&module, RuntimeOptions::default())
            .expect("Could not create wrapper");
        let value: usize = module.get("value").expect("Could not get value");
        assert_eq!(3, value);
    }

    #[test]
    fn test_callable() {
        let module = Module::new(
            "test.js",
            "
            export const value = 3;
            export function func() { return 4; }
        ",
        );

        let mut module = ModuleWrapper::new_from_module(&module, RuntimeOptions::default())
            .expect("Could not create wrapper");

        assert!(module.is_callable("func"));
        assert!(!module.is_callable("value"));
    }

    #[test]
    fn test_keys() {
        let module = Module::new(
            "test.js",
            "
            export const value = 3;
            export function func() { return 4; }
        ",
        );

        let mut module = ModuleWrapper::new_from_module(&module, RuntimeOptions::default())
            .expect("Could not create wrapper");
        let mut keys = module.keys();
        assert_eq!(2, keys.len());
        assert_eq!("value", keys.pop().unwrap());
        assert_eq!("func", keys.pop().unwrap());
    }
}
