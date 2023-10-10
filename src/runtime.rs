use deno_core::serde_json;

use crate::{
    inner_runtime::{InnerRuntime, InnerRuntimeOptions},
    Error, JsFunction, Module, ModuleHandle,
};

/// Represents the set of options accepted by the runtime constructor
pub type RuntimeOptions = InnerRuntimeOptions;

/// For functions returning nothing
pub type Undefined = serde_json::Value;

/// Represents a configured runtime ready to run modules
pub struct Runtime(InnerRuntime);

impl Runtime {
    /// The lack of any arguments - used to simplify calling functions
    /// Prevents you from needing to specify the type using ::<serde_json::Value>
    pub const EMPTY_ARGS: &'static [serde_json::Value] = &[];

    /// Creates a new instance of the runtime with the provided options.
    ///
    /// # Arguments
    /// * `options` - A `RuntimeOptions` struct that specifies the configuration options for the runtime.
    ///
    /// # Returns
    /// A `Result` containing either the initialized runtime instance on success (`Ok`) or an error on failure (`Err`).
    ///
    /// # Example
    /// ```rust
    /// use js_playground::{ json_args, Runtime, RuntimeOptions, Module };
    /// use std::time::Duration;
    ///
    /// # fn main() -> Result<(), js_playground::Error> {
    /// // Creates a runtime that will attempt to run function load() on start
    /// // And which will time-out after 50ms
    /// let mut runtime = Runtime::new(RuntimeOptions {
    ///     default_entrypoint: Some("load".to_string()),
    ///     timeout: Duration::from_millis(50),
    ///     ..Default::default()
    /// })?;
    ///
    /// let module = Module::new("test.js", "
    ///     export const load = () => {
    ///         return 'Hello World!';
    ///     }
    /// ");
    ///
    /// let module_handle = runtime.load_module(&module)?;
    /// let value: String = runtime.call_entrypoint(&module_handle, json_args!())?;
    /// assert_eq!("Hello World!", value);
    /// # Ok(())
    /// # }
    /// ```
    ///
    pub fn new(options: RuntimeOptions) -> Result<Self, Error> {
        Ok(Self(InnerRuntime::new(options)))
    }

    /// Access the underlying deno runtime instance directly
    pub fn deno_runtime(&mut self) -> &mut deno_core::JsRuntime {
        self.0.deno_runtime()
    }

    /// Access the options used to create this runtime
    pub fn options(&self) -> &RuntimeOptions {
        &self.0.options
    }

    /// Encode an argument as a json value for use as a function argument
    /// ```rust
    /// use js_playground::{ Runtime, RuntimeOptions, Module };
    /// use std::time::Duration;
    ///
    /// # fn main() -> Result<(), js_playground::Error> {
    /// let module = Module::new("test.js", "
    ///     function load(a, b) {
    ///         console.log(`Hello world: a=${a}, b=${b}`);
    ///     }
    ///     js_playground.register_entrypoint(load);
    /// ");
    ///
    /// Runtime::execute_module(
    ///     &module, vec![],
    ///     Default::default(),
    ///     &[
    ///         Runtime::arg("test"),
    ///         Runtime::arg(5),
    ///     ]
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn arg<A>(value: A) -> serde_json::Value
    where
        serde_json::Value: From<A>,
    {
        serde_json::Value::from(value)
    }

    /// Calls a stored javascript function and deserializes its return value.
    ///
    /// # Arguments
    /// * `function` - A The function object
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function cannot be found, if there are issues with
    /// calling the function, or if the result cannot be deserialized.
    pub fn call_stored_function<T>(
        &mut self,
        module_context: &ModuleHandle,
        function: &JsFunction,
        args: &[serde_json::Value],
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        self.0.call_stored_function(module_context, function, args)
    }

    /// Calls a javascript function within the Deno runtime by its name and deserializes its return value.
    ///
    /// # Arguments
    /// * `name` - A string representing the name of the javascript function to call.
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function cannot be found, if there are issues with
    /// calling the function, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::{ json_args, Runtime, Module, Error };
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("/path/to/module.js", "export function f() { return 2; };");
    /// let module = runtime.load_module(&module)?;
    /// let value: usize = runtime.call_function(&module, "f", json_args!())?;
    /// # Ok(())
    /// # }
    /// ```
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
    ///
    /// # Returns
    /// A `Result` containing the deserialized result or an error (`Error`) if the
    /// value cannot be found, if there are issues with, or if the result cannot be
    ///  deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::{ Runtime, Module, Error };
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("/path/to/module.js", "globalThis.my_value = 2;");
    /// let module = runtime.load_module(&module)?;
    /// let value: usize = runtime.get_value(&module, "my_value")?;
    /// # Ok(())
    /// # }
    /// ```
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
    ///
    /// # Returns
    /// A `Result` containing a handle for the loaded module
    /// or an error (`Error`) if there are issues with loading modules, executing the
    /// module, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create a module with filename and contents
    /// use js_playground::{Runtime, Module, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("test.js", "js_playground.register_entrypoint(() => 'test')");
    /// runtime.load_module(&module);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_module(&mut self, module: &Module) -> Result<ModuleHandle, Error> {
        self.0.load_modules(None, vec![module])
    }

    /// Executes the given module, and returns a handle allowing you to extract values
    /// And call functions.
    ///
    /// This will load 'module' as the main module, and the others as side-modules.
    /// Only one main module can be loaded, so be sure to call `.reset()` if you need
    /// to load a different main module.
    ///
    /// # Arguments
    /// * `module` - A `Module` object containing the module's filename and contents.
    /// * `side_modules` - A set of additional modules to be loaded into memory for use
    ///
    /// # Returns
    /// A `Result` containing a handle for the loaded module
    /// or an error (`Error`) if there are issues with loading modules, executing the
    /// module, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create a module with filename and contents
    /// use js_playground::{Runtime, Module, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("test.js", "js_playground.register_entrypoint(() => 'test')");
    /// runtime.load_modules(&module, vec![]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_modules(
        &mut self,
        module: &Module,
        side_modules: Vec<&Module>,
    ) -> Result<ModuleHandle, Error> {
        self.0.load_modules(Some(&module), side_modules)
    }

    /// Executes the entrypoint function of a module within the Deno runtime.
    ///
    /// # Arguments
    /// * `module_context` - A handle returned by loading a module into the runtime
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the entrypoint execution (`T`)
    /// if successful, or an error (`Error`) if the entrypoint is missing, the execution fails,
    /// or the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::{json_args, Runtime, Module, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("test.js", "js_playground.register_entrypoint(() => 'test')");
    /// let module = runtime.load_module(&module)?;
    ///
    /// // Run the entrypoint and handle the result
    /// let value: String = runtime.call_entrypoint(&module, json_args!())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn call_entrypoint<T>(
        &mut self,
        module_context: &ModuleHandle,
        args: &[serde_json::Value],
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        if let Some(entrypoint) = module_context.entrypoint() {
            let value: serde_json::Value =
                self.0
                    .call_function_by_ref_async(module_context, entrypoint.clone(), args)?;
            Ok(serde_json::from_value(value)?)
        } else {
            Err(Error::MissingEntrypoint(module_context.module().clone()))
        }
    }

    /// Loads a module into a new runtime, executes the entry function and returns the
    /// result of the module's execution, deserialized into the specified Rust type (`T`).
    ///
    /// # Arguments
    /// * `module` - A `Module` object containing the module's filename and contents.
    /// * `side_modules` - A set of additional modules to be loaded into memory for use
    /// * `runtime_options` - Options for the creation of the runtime
    /// * `entrypoint_args` - Arguments to pass to the entrypoint function
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the entrypoint execution (`T`)
    /// if successful, or an error (`Error`) if the entrypoint is missing, the execution fails,
    /// or the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create a module with filename and contents
    /// use js_playground::{json_args, Runtime, Module, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// let module = Module::new("test.js", "js_playground.register_entrypoint(() => 2)");
    /// let value: usize = Runtime::execute_module(&module, vec![], Default::default(), json_args!())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute_module<T>(
        module: &Module,
        side_modules: Vec<&Module>,
        runtime_options: RuntimeOptions,
        entrypoint_args: &[serde_json::Value],
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        let mut runtime = Runtime::new(runtime_options)?;
        let module = runtime.load_modules(module, side_modules)?;
        let value: T = runtime.call_entrypoint(&module, entrypoint_args)?;
        Ok(value)
    }

    /// Reset the runtime
    /// This clears any side-effects in global, and unloads any running modules
    ///
    /// Use this function if you need to clear the sandbox between runs, to prevent
    /// interop side-effects
    pub fn reset(&mut self) {
        self.0.clear_modules();
        self.call_function::<Undefined>(
            &ModuleHandle::default(),
            "js_playground_reset",
            Runtime::EMPTY_ARGS,
        )
        .expect("Could not reset the runtime");
    }
}

#[cfg(test)]
mod test_runtime {
    use crate::json_args;
    use std::time::Duration;

    use super::*;
    use deno_core::extension;

    #[test]
    fn test_new() {
        Runtime::new(Default::default()).expect("Could not create the runtime");

        extension!(test_extension);
        Runtime::new(RuntimeOptions {
            extensions: vec![test_extension::init_ops_and_esm()],
            ..Default::default()
        })
        .expect("Could not create runtime with extensions");
    }

    #[test]
    fn test_arg() {
        assert_eq!(2, Runtime::arg(2));
        assert_eq!("test", Runtime::arg("test"));
        assert_ne!("test", Runtime::arg(2));
    }

    #[test]
    fn test_get_value() {
        let module = Module::new(
            "test.js",
            "
            globalThis.a = 2;
            export const b = 'test';
            export const fnc = null;
        ",
        );

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = runtime
            .load_modules(&module, vec![])
            .expect("Could not load module");

        assert_eq!(
            2,
            runtime
                .get_value::<usize>(&module, "a")
                .expect("Could not find global")
        );
        assert_eq!(
            "test",
            runtime
                .get_value::<String>(&module, "b")
                .expect("Could not find export")
        );
        runtime
            .get_value::<Undefined>(&module, "c")
            .expect_err("Could not detect null");
        runtime
            .get_value::<Undefined>(&module, "d")
            .expect_err("Could not detect undeclared");
    }

    #[test]
    fn test_load_module() {
        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            js_playground.register_entrypoint(() => 2);
        ",
        );
        let module = runtime
            .load_modules(&module, vec![])
            .expect("Could not load module");
        assert_ne!(0, module.id());

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module1 = Module::new(
            "importme.js",
            "
            export const value = 2;
        ",
        );
        let module2 = Module::new(
            "test.js",
            "
            import { value } from './importme.js';
            js_playground.register_entrypoint(() => value);
        ",
        );
        runtime
            .load_module(&module1)
            .expect("Could not load modules");
        let module = runtime
            .load_module(&module2)
            .expect("Could not load modules");
        let value: usize = runtime
            .call_entrypoint(&module, json_args!())
            .expect("Could not call exported fn");
        assert_eq!(2, value);

        let mut runtime = Runtime::new(RuntimeOptions {
            timeout: Duration::from_millis(50),
            ..Default::default()
        })
        .expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            await new Promise(r => setTimeout(r, 2000));
        ",
        );
        runtime
            .load_modules(&module, vec![])
            .expect_err("Did not interupt after timeout");
    }

    #[test]
    fn test_load_modules() {
        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            js_playground.register_entrypoint(() => 2);
        ",
        );
        let module = runtime
            .load_modules(&module, vec![])
            .expect("Could not load module");
        assert_ne!(0, module.id());

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module1 = Module::new(
            "importme.js",
            "
            export const value = 2;
        ",
        );
        let module2 = Module::new(
            "test.js",
            "
            import { value } from './importme.js';
            js_playground.register_entrypoint(() => value);
        ",
        );
        let module = runtime
            .load_modules(&module2, vec![&module1])
            .expect("Could not load modules");
        let value: usize = runtime
            .call_entrypoint(&module, json_args!())
            .expect("Could not call exported fn");
        assert_eq!(2, value);

        let mut runtime = Runtime::new(RuntimeOptions {
            timeout: Duration::from_millis(50),
            ..Default::default()
        })
        .expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            await new Promise(r => setTimeout(r, 5000));
        ",
        );
        runtime
            .load_modules(&module, vec![])
            .expect_err("Did not interupt after timeout");
    }

    #[test]
    fn test_call_entrypoint() {
        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            js_playground.register_entrypoint(() => 2);
        ",
        );
        let module = runtime
            .load_modules(&module, vec![])
            .expect("Could not load module");
        let value: usize = runtime
            .call_entrypoint(&module, json_args!())
            .expect("Could not call registered fn");
        assert_eq!(2, value);

        let mut runtime = Runtime::new(RuntimeOptions {
            default_entrypoint: Some("load".to_string()),
            ..Default::default()
        })
        .expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            export const load = () => 2;
        ",
        );
        let module = runtime
            .load_modules(&module, vec![])
            .expect("Could not load module");
        let value: usize = runtime
            .call_entrypoint(&module, json_args!())
            .expect("Could not call exported fn");
        assert_eq!(2, value);

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = Module::new(
            "test.js",
            "
            export const load = () => 2;
        ",
        );
        let module = runtime
            .load_modules(&module, vec![])
            .expect("Could not load module");
        runtime
            .call_entrypoint::<Undefined>(&module, json_args!())
            .expect_err("Did not detect no entrypoint");
    }

    #[test]
    fn test_execute_module() {
        let module = Module::new(
            "test.js",
            "
            js_playground.register_entrypoint(() => 2);
        ",
        );
        let value: usize =
            Runtime::execute_module(&module, vec![], Default::default(), json_args!())
                .expect("Could not exec module");
        assert_eq!(2, value);

        let module = Module::new(
            "test.js",
            "
            function load() { return 2; }
        ",
        );
        Runtime::execute_module::<Undefined>(&module, vec![], Default::default(), json_args!())
            .expect_err("Could not detect no entrypoint");
    }

    #[test]
    fn test_reset() {
        let module = Module::new(
            "test.js",
            "
            js_playground.register_entrypoint(() => globalThis.foo = 'bar');
            export const getFoo = () => globalThis.foo;
        ",
        );

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = runtime
            .load_modules(&module, vec![])
            .expect("Could not load module");
        runtime
            .call_entrypoint::<Undefined>(&module, json_args!())
            .expect("Could not call entrypoint");

        assert_eq!(
            "bar",
            runtime
                .call_function::<String>(&module, "getFoo", json_args!())
                .expect("Error getting value")
        );

        runtime.reset();
        runtime
            .call_function::<String>(&module, "getFoo", json_args!())
            .expect_err("Global was not cleared");
    }

    #[test]
    fn call_function() {
        let module = Module::new(
            "test.js",
            "
            globalThis.fna = (i) => i;
            export function fnb() { return 'test'; }
            export const fnc = 2;
            export const fne = () => {};
        ",
        );

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = runtime
            .load_modules(&module, vec![])
            .expect("Could not load module");

        let result: usize = runtime
            .call_function(&module, "fna", &[Runtime::arg(2)])
            .expect("Could not call global");
        assert_eq!(2, result);

        let result: String = runtime
            .call_function(&module, "fnb", json_args!())
            .expect("Could not call export");
        assert_eq!("test", result);

        runtime
            .call_function::<Undefined>(&module, "fnc", json_args!())
            .expect_err("Did not detect non-function");
        runtime
            .call_function::<Undefined>(&module, "fnd", json_args!())
            .expect_err("Did not detect undefined");
        runtime
            .call_function::<Undefined>(&module, "fne", json_args!())
            .expect("Did not allow undefined return");
    }
}
