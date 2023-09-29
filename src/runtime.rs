use tokio::runtime;

use deno_core::serde_json;
use deno_core::v8;
use deno_core::Extension;
use deno_core::FsModuleLoader;
use deno_core::JsRuntime;

use std::rc::Rc;
use std::time::Duration;

use crate::ext::*;
use crate::script::Script;
use crate::traits::*;
use crate::Error;
use crate::ModuleHandle;

#[cfg(feature = "web")]
#[derive(Clone)]
struct Permissions;
#[cfg(feature = "web")]
impl deno_web::TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        false
    }
    fn check_unstable(&self, _state: &OpState, _api_name: &'static str) {
        unreachable!()
    }
}

#[derive(Default)]
/// Represents the set of options accepted by the runtime constructor
pub struct RuntimeOptions {
    /// A set of deno_core extensions to add to the runtime
    pub extensions: Vec<deno_core::Extension>,

    /// Function to use as entrypoint if the script does not provide one
    pub default_entrypoint: Option<String>,

    /// Amount of time to run for before killing the thread
    pub timeout: Option<Duration>,
}

/// For functions returning nothing
pub type Undefined = serde_json::Value;

/// Represents a configured runtime ready to run modules
pub struct Runtime {
    deno_runtime: JsRuntime,
    options: RuntimeOptions,
}

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
    /// use js_playground::{ Runtime, RuntimeOptions, Script };
    /// use std::time::Duration;
    ///
    /// # fn main() -> Result<(), js_playground::Error> {
    /// // Creates a runtime that will attempt to run function load() on start
    /// // And which will time-out after 50ms
    /// let mut runtime = Runtime::new(RuntimeOptions {
    ///     default_entrypoint: Some("load".to_string()),
    ///     timeout: Some(Duration::from_millis(50)),
    ///     ..Default::default()
    /// })?;
    ///
    /// let script = Script::new("test.js", "
    ///     export const load = () => {
    ///         return 'Hello World!';
    ///     }
    /// ");
    ///
    /// let module_handle = runtime.load_module(&script)?;
    /// let value: String = runtime.call_entrypoint(&module_handle, Runtime::EMPTY_ARGS)?;
    /// assert_eq!("Hello World!", value);
    /// # Ok(())
    /// # }
    /// ```
    ///
    pub fn new(options: RuntimeOptions) -> Result<Self, Error> {
        let js_runtime = JsRuntime::new(deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(FsModuleLoader)),
            extensions: Runtime::all_extensions(options.extensions),
            ..Default::default()
        });

        Ok(Self {
            deno_runtime: js_runtime,
            options: RuntimeOptions {
                default_entrypoint: options.default_entrypoint.clone(),
                timeout: options.timeout,
                ..Default::default()
            },
        })
    }

    /// Access the underlying deno runtime instance directly
    pub fn deno_runtime(&mut self) -> &mut JsRuntime {
        &mut self.deno_runtime
    }

    /// Access the options used to create this runtime
    pub fn options(&self) -> &RuntimeOptions {
        &self.options
    }

    /// Encode an argument as a json value for use as a function argument
    /// ```rust
    /// use js_playground::{ Runtime, RuntimeOptions, Script };
    /// use std::time::Duration;
    ///
    /// # fn main() -> Result<(), js_playground::Error> {
    /// let script = Script::new("test.js", "
    ///     function load(a, b) {
    ///         console.log(`Hello world: a=${a}, b=${b}`);
    ///     }
    ///     js_playground.register_entrypoint(load);
    /// ");
    ///
    /// Runtime::execute_module(
    ///     &script, vec![],
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

    /// Calls a JavaScript function within the Deno runtime by its name and deserializes its return value.
    ///
    /// # Arguments
    /// * `name` - A string representing the name of the JavaScript function to call.
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function cannot be found, if there are issues with
    /// calling the function, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::{ Runtime, Script, Error };
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let script = Script::new("/path/to/module.js", "export function f() { return 2; };");
    /// let module = runtime.load_module(&script)?;
    /// let value: usize = runtime.call_function(&module, "f", Runtime::EMPTY_ARGS)?;
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
        let function = self.get_function_by_name(module_context, name)?;
        self.call_function_by_ref(module_context, function, args)
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
    /// use js_playground::{ Runtime, Script, Error };
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let script = Script::new("/path/to/module.js", "globalThis.my_value = 2;");
    /// let module = runtime.load_module(&script)?;
    /// let value: usize = runtime.get_value(&module, "my_value")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_value<T>(&mut self, module_context: &ModuleHandle, name: &str) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        let value = self.get_value_ref(module_context, name)?;
        let mut scope = self.deno_runtime.handle_scope();
        let local_value = v8::Local::<v8::Value>::new(&mut scope, value);
        Ok(deno_core::serde_v8::from_v8(&mut scope, local_value)?)
    }

    /// Executes the given script, and returns a handle allowing you to extract values
    /// And call functions
    ///
    /// # Arguments
    /// * `module` - A `Script` object containing the module's filename and contents.
    ///
    /// # Returns
    /// A `Result` containing a handle for the loaded module
    /// or an error (`Error`) if there are issues with loading modules, executing the
    /// script, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create a script with filename and contents
    /// use js_playground::{Runtime, Script, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let script = Script::new("test.js", "js_playground.register_entrypoint(() => 'test')");
    /// runtime.load_module(&script);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_module(&mut self, module: &Script) -> Result<ModuleHandle, Error> {
        self.load_modules_inner(None, vec![module])
    }

    /// Executes the given script, and returns a handle allowing you to extract values
    /// And call functions.
    ///
    /// This will load 'module' as the main module, and the others as side-modules.
    /// Only one main module can be loaded, so be sure to call `.reset()` if you need
    /// to load a different main module.
    ///
    /// # Arguments
    /// * `module` - A `Script` object containing the module's filename and contents.
    /// * `side_modules` - A set of additional modules to be loaded into memory for use
    ///
    /// # Returns
    /// A `Result` containing a handle for the loaded module
    /// or an error (`Error`) if there are issues with loading modules, executing the
    /// script, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create a script with filename and contents
    /// use js_playground::{Runtime, Script, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let script = Script::new("test.js", "js_playground.register_entrypoint(() => 'test')");
    /// runtime.load_modules(&script, vec![]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_modules(
        &mut self,
        module: &Script,
        side_modules: Vec<&Script>,
    ) -> Result<ModuleHandle, Error> {
        self.load_modules_inner(Some(module), side_modules)
    }

    /// Executes the entrypoint function of a script within the Deno runtime.
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
    /// use js_playground::{Runtime, Script, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let script = Script::new("test.js", "js_playground.register_entrypoint(() => 'test')");
    /// let module = runtime.load_module(&script)?;
    ///
    /// // Run the entrypoint and handle the result
    /// let value: String = runtime.call_entrypoint(&module, Runtime::EMPTY_ARGS)?;
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
                self.call_function_by_ref(module_context, entrypoint.clone(), args)?;
            Ok(serde_json::from_value(value)?)
        } else {
            Err(Error::MissingEntrypoint(module_context.module().clone()))
        }
    }

    /// Loads a module into a new runtime, executes the entry function and returns the
    /// result of the script's execution, deserialized into the specified Rust type (`T`).
    ///
    /// # Arguments
    /// * `module` - A `Script` object containing the module's filename and contents.
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
    /// // Create a script with filename and contents
    /// use js_playground::{Runtime, Script, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// let script = Script::new("test.js", "js_playground.register_entrypoint(() => 2)");
    /// let value: usize = Runtime::execute_module(&script, vec![], Default::default(), Runtime::EMPTY_ARGS)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute_module<T>(
        module: &Script,
        side_modules: Vec<&Script>,
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
        // self.deno_runtime.clear_modules();
        self.call_function::<Undefined>(
            &ModuleHandle::default(),
            "js_playground_reset",
            Runtime::EMPTY_ARGS,
        )
        .expect("Could not reset the runtime");
    }

    /// Load one or more modules
    ///
    /// Will return a handle to the main module, or the last
    /// side-module
    fn load_modules_inner(
        &mut self,
        main_module: Option<&Script>,
        side_modules: Vec<&Script>,
    ) -> Result<ModuleHandle, Error> {
        if main_module.is_none() && side_modules.is_empty() {
            return Err(Error::Runtime(
                "Internal error: attempt to load no modules".to_string(),
            ));
        }

        // Evaluate the script
        let deno_runtime = &mut self.deno_runtime;
        let future = async move {
            let mut module_handle_stub = Default::default();

            // Get additional modules first
            for side_module in side_modules {
                let s_modid = deno_runtime
                    .load_side_module(
                        &side_module.filename().to_module_specifier()?,
                        Some(deno_core::FastString::from(
                            side_module.contents().to_string(),
                        )),
                    )
                    .await?;
                let result = deno_runtime.mod_evaluate(s_modid);
                deno_runtime.run_event_loop(false).await?;
                result.await??;
                module_handle_stub = ModuleHandle::new(&side_module, s_modid, None);
            }

            // Load main module
            if let Some(module) = main_module {
                let module_id = deno_runtime
                    .load_main_module(
                        &module.filename().to_module_specifier()?,
                        Some(deno_core::FastString::from(module.contents().to_string())),
                    )
                    .await?;

                // Finish execution
                let result = deno_runtime.mod_evaluate(module_id);
                deno_runtime.run_event_loop(false).await?;
                result.await??;
                module_handle_stub = ModuleHandle::new(&module, module_id, None);
            }

            Ok::<ModuleHandle, deno_core::anyhow::Error>(module_handle_stub)
        };

        // Get the thread ready
        let tokio_runtime = runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        // Let it play out...
        let module_handle_stub = tokio_runtime.block_on(future)?;
        if let Some(timeout) = self.options.timeout {
            tokio_runtime.shutdown_timeout(timeout);
        }

        // Try to get an entrypoint
        let state = self.deno_runtime().op_state();
        let mut deep_state = state.try_borrow_mut()?;
        let f_entrypoint = match deep_state.try_take::<v8::Global<v8::Function>>() {
            Some(entrypoint) => Some(entrypoint),
            None => self
                .options
                .default_entrypoint
                .clone()
                .and_then(|default_entrypoint| {
                    self.get_function_by_name(&module_handle_stub, &default_entrypoint)
                        .ok()
                }),
        };

        Ok(ModuleHandle::new(
            module_handle_stub.module(),
            module_handle_stub.id(),
            f_entrypoint,
        ))
    }

    /// Attempt to get a value out of the global context (globalThis.name)
    ///
    /// # Arguments
    /// * `name` - Name of the object to extract
    ///
    /// # Returns
    /// A `Result` containing the non-null value extracted or an error (`Error`)
    fn get_global_value(&mut self, name: &str) -> Result<v8::Global<v8::Value>, Error> {
        let context = self.deno_runtime.main_context();
        let mut scope = self.deno_runtime.handle_scope();
        let global = context.open(&mut scope).global(&mut scope);

        let key = name.to_v8_string(&mut scope)?;
        let value = global.get(&mut scope, key.into());

        match value.if_defined() {
            Some(v) => Ok(v8::Global::<v8::Value>::new(&mut scope, v)),
            _ => Err(Error::ValueNotFound(name.to_string())),
        }
    }

    /// Attempt to get a value out of a module context (export ...)
    ///
    /// # Arguments
    /// * `module` - A handle to a loaded module
    /// * `name` - Name of the object to extract
    ///
    /// # Returns
    /// A `Result` containing the non-null value extracted or an error (`Error`)
    fn get_module_export_value(
        &mut self,
        module: &ModuleHandle,
        name: &str,
    ) -> Result<v8::Global<v8::Value>, Error> {
        let module_namespace = self.deno_runtime.get_module_namespace(module.id())?;
        let mut scope = self.deno_runtime.handle_scope();
        let module_namespace = module_namespace.open(&mut scope);
        assert!(module_namespace.is_module_namespace_object());

        let key = name.to_v8_string(&mut scope)?;
        let value = module_namespace.get(&mut scope, key.into());

        match value.if_defined() {
            Some(v) => Ok(v8::Global::<v8::Value>::new(&mut scope, v)),
            _ => Err(Error::ValueNotFound(name.to_string())),
        }
    }

    /// Attempt to get a value out of a runtime
    ///
    /// # Arguments
    /// * `module` - A handle to a loaded module
    /// * `name` - Name of the object to extract
    ///
    /// # Returns
    /// A `Result` containing the non-null value extracted or an error (`Error`)
    fn get_value_ref(
        &mut self,
        module: &ModuleHandle,
        name: &str,
    ) -> Result<v8::Global<v8::Value>, Error> {
        match self.get_global_value(name) {
            Ok(v) => Some(v),
            _ => self.get_module_export_value(module, name).ok(),
        }
        .ok_or::<Error>(Error::ValueNotFound(name.to_string()))
    }

    /// This method takes a JavaScript function and invokes it within the Deno runtime.
    /// It then serializes the return value of the function into a JSON string and
    /// deserializes it into the specified Rust type (`T`).
    ///
    /// # Arguments
    /// * `function` - A reference to a JavaScript function (`v8::Function`)
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function call fails or the return value cannot
    /// be deserialized.
    fn call_function_by_ref<T>(
        &mut self,
        module_context: &ModuleHandle,
        function: v8::Global<v8::Function>,
        args: &[serde_json::Value],
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        let module_namespace = self
            .deno_runtime
            .get_module_namespace(module_context.id())?;
        let mut scope = self.deno_runtime.handle_scope();
        let module_namespace = v8::Local::<v8::Object>::new(&mut scope, module_namespace);
        let function_instance = function.open(&mut scope);

        // Prep arguments
        let f_args: Result<Vec<v8::Local<v8::Value>>, deno_core::serde_v8::Error> = args
            .iter()
            .map(|f| deno_core::serde_v8::to_v8(&mut scope, f))
            .collect();
        let final_args = f_args?;

        // Call the function
        let result = function_instance
            .call(&mut scope, module_namespace.into(), &final_args)
            .unwrap_or(deno_core::serde_v8::to_v8(
                &mut scope,
                serde_json::Value::Null,
            )?);

        // Decode value
        let value: T = deno_core::serde_v8::from_v8(&mut scope, result)?;
        Ok(value)
    }

    /// Retrieves a JavaScript function by its name from the Deno runtime's global context.
    ///
    /// # Arguments
    /// * `name` - A string representing the name of the JavaScript function to retrieve.
    ///
    /// # Returns
    /// A `Result` containing a `v8::Global<v8::Function>` if
    /// the function is found, or an error (`Error`) if the function cannot be found or
    /// if it is not a valid JavaScript function.
    fn get_function_by_name(
        &mut self,
        module_context: &ModuleHandle,
        name: &str,
    ) -> Result<v8::Global<v8::Function>, Error> {
        // Get the value
        let value = self.get_value_ref(module_context, name)?;

        // Convert it into a function
        let mut scope = self.deno_runtime.handle_scope();
        let local_value = v8::Local::<v8::Value>::new(&mut scope, value);
        let f: v8::Local<v8::Function> = local_value
            .try_into()
            .or::<Error>(Err(Error::ValueNotCallable(name.to_string())))?;

        // Return it as a global
        Ok(v8::Global::<v8::Function>::new(&mut scope, f))
    }

    /// Adds internal extensions to the list provided by the user
    ///
    /// # Arguments
    /// * `user_extensions` - A set of deno_core::Extension objects provided by the user
    fn all_extensions(mut user_extensions: Vec<Extension>) -> Vec<Extension> {
        user_extensions.extend(vec![js_playground::js_playground::init_ops_and_esm()]);

        #[cfg(feature = "console")]
        user_extensions.extend(vec![
            deno_console::deno_console::init_ops_and_esm(),
            init_console::init_console::init_ops_and_esm(),
        ]);

        #[cfg(feature = "url")]
        user_extensions.extend(vec![
            deno_webidl::deno_webidl::init_ops_and_esm(),
            deno_url::deno_url::init_ops_and_esm(),
            init_url::init_url::init_ops_and_esm(),
        ]);

        #[cfg(feature = "web")]
        user_extensions.extend(vec![deno_web::deno_web::init_ops_and_esm::<Permissions>(
            Default::default(),
            None,
        )]);

        user_extensions
    }
}

#[cfg(test)]
mod test_runtime {
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
        let script = Script::new(
            "test.js",
            "
            globalThis.a = 2;
            export const b = 'test';
            export const fnc = null;
        ",
        );

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = runtime
            .load_modules(&script, vec![])
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
        let script = Script::new(
            "test.js",
            "
            js_playground.register_entrypoint(() => 2);
        ",
        );
        let module = runtime
            .load_modules(&script, vec![])
            .expect("Could not load module");
        assert_ne!(0, module.id());

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let script1 = Script::new(
            "importme.js",
            "
            export const value = 2;
        ",
        );
        let script2 = Script::new(
            "test.js",
            "
            import { value } from './importme.js';
            js_playground.register_entrypoint(() => value);
        ",
        );
        runtime
            .load_module(&script1)
            .expect("Could not load modules");
        let module = runtime
            .load_module(&script2)
            .expect("Could not load modules");
        let value: usize = runtime
            .call_entrypoint(&module, Runtime::EMPTY_ARGS)
            .expect("Could not call exported fn");
        assert_eq!(2, value);

        let mut runtime = Runtime::new(RuntimeOptions {
            timeout: Some(Duration::from_millis(50)),
            ..Default::default()
        })
        .expect("Could not create the runtime");
        let script = Script::new(
            "test.js",
            "
            await new Promise(r => setTimeout(r, 2000));
        ",
        );
        runtime
            .load_modules(&script, vec![])
            .expect_err("Did not interupt after timeout");
    }

    #[test]
    fn test_load_modules() {
        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let script = Script::new(
            "test.js",
            "
            js_playground.register_entrypoint(() => 2);
        ",
        );
        let module = runtime
            .load_modules(&script, vec![])
            .expect("Could not load module");
        assert_ne!(0, module.id());

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let script1 = Script::new(
            "importme.js",
            "
            export const value = 2;
        ",
        );
        let script2 = Script::new(
            "test.js",
            "
            import { value } from './importme.js';
            js_playground.register_entrypoint(() => value);
        ",
        );
        let module = runtime
            .load_modules(&script2, vec![&script1])
            .expect("Could not load modules");
        let value: usize = runtime
            .call_entrypoint(&module, Runtime::EMPTY_ARGS)
            .expect("Could not call exported fn");
        assert_eq!(2, value);

        let mut runtime = Runtime::new(RuntimeOptions {
            timeout: Some(Duration::from_millis(50)),
            ..Default::default()
        })
        .expect("Could not create the runtime");
        let script = Script::new(
            "test.js",
            "
            await new Promise(r => setTimeout(r, 2000));
        ",
        );
        runtime
            .load_modules(&script, vec![])
            .expect_err("Did not interupt after timeout");
    }

    #[test]
    fn test_call_entrypoint() {
        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let script = Script::new(
            "test.js",
            "
            js_playground.register_entrypoint(() => 2);
        ",
        );
        let module = runtime
            .load_modules(&script, vec![])
            .expect("Could not load module");
        let value: usize = runtime
            .call_entrypoint(&module, Runtime::EMPTY_ARGS)
            .expect("Could not call registered fn");
        assert_eq!(2, value);

        let mut runtime = Runtime::new(RuntimeOptions {
            default_entrypoint: Some("load".to_string()),
            ..Default::default()
        })
        .expect("Could not create the runtime");
        let script = Script::new(
            "test.js",
            "
            export const load = () => 2;
        ",
        );
        let module = runtime
            .load_modules(&script, vec![])
            .expect("Could not load module");
        let value: usize = runtime
            .call_entrypoint(&module, Runtime::EMPTY_ARGS)
            .expect("Could not call exported fn");
        assert_eq!(2, value);

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let script = Script::new(
            "test.js",
            "
            export const load = () => 2;
        ",
        );
        let module = runtime
            .load_modules(&script, vec![])
            .expect("Could not load module");
        runtime
            .call_entrypoint::<Undefined>(&module, Runtime::EMPTY_ARGS)
            .expect_err("Did not detect no entrypoint");
    }

    #[test]
    fn test_execute_module() {
        let script = Script::new(
            "test.js",
            "
            js_playground.register_entrypoint(() => 2);
        ",
        );
        let value: usize =
            Runtime::execute_module(&script, vec![], Default::default(), Runtime::EMPTY_ARGS)
                .expect("Could not exec module");
        assert_eq!(2, value);

        let script = Script::new(
            "test.js",
            "
            function load() { return 2; }
        ",
        );
        Runtime::execute_module::<Undefined>(
            &script,
            vec![],
            Default::default(),
            Runtime::EMPTY_ARGS,
        )
        .expect_err("Could not detect no entrypoint");
    }

    #[test]
    fn test_reset() {
        let script = Script::new(
            "test.js",
            "
            js_playground.register_entrypoint(() => globalThis.foo = 'bar');
            export const getFoo = () => globalThis.foo;
        ",
        );

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = runtime
            .load_modules(&script, vec![])
            .expect("Could not load module");
        runtime
            .call_entrypoint::<Undefined>(&module, Runtime::EMPTY_ARGS)
            .expect("Could not call entrypoint");

        assert_eq!(
            "bar",
            runtime
                .call_function::<String>(&module, "getFoo", Runtime::EMPTY_ARGS)
                .expect("Error getting value")
        );

        runtime.reset();
        runtime
            .call_function::<String>(&module, "getFoo", Runtime::EMPTY_ARGS)
            .expect_err("Global was not cleared");
    }

    #[test]
    fn test_get_value_by_ref() {
        let script = Script::new(
            "test.js",
            "
            globalThis.a = 2;
            export const b = 'test';
            export const fnc = null;
        ",
        );

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = runtime
            .load_modules(&script, vec![])
            .expect("Could not load module");

        runtime
            .get_value_ref(&module, "a")
            .expect("Could not find global");
        runtime
            .get_value_ref(&module, "b")
            .expect("Could not find export");
        runtime
            .get_value_ref(&module, "c")
            .expect_err("Could not detect null");
        runtime
            .get_value_ref(&module, "d")
            .expect_err("Could not detect undeclared");
    }

    #[test]
    fn call_function() {
        let script = Script::new(
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
            .load_modules(&script, vec![])
            .expect("Could not load module");

        let result: usize = runtime
            .call_function(&module, "fna", &[Runtime::arg(2)])
            .expect("Could not call global");
        assert_eq!(2, result);

        let result: String = runtime
            .call_function(&module, "fnb", Runtime::EMPTY_ARGS)
            .expect("Could not call export");
        assert_eq!("test", result);

        runtime
            .call_function::<Undefined>(&module, "fnc", Runtime::EMPTY_ARGS)
            .expect_err("Did not detect non-function");
        runtime
            .call_function::<Undefined>(&module, "fnd", Runtime::EMPTY_ARGS)
            .expect_err("Did not detect undefined");
        runtime
            .call_function::<Undefined>(&module, "fne", Runtime::EMPTY_ARGS)
            .expect("Did not allow undefined return");
    }

    #[test]
    fn test_get_function_by_name() {
        let script = Script::new(
            "test.js",
            "
            globalThis.fna = () => {};
            export function fnb() {}
            export const fnc = 2;
        ",
        );

        let mut runtime = Runtime::new(Default::default()).expect("Could not create the runtime");
        let module = runtime
            .load_modules(&script, vec![])
            .expect("Could not load module");

        runtime
            .get_function_by_name(&module, "fna")
            .expect("Did not find global");
        runtime
            .get_function_by_name(&module, "fnb")
            .expect("Did not find export");
        runtime
            .get_function_by_name(&module, "fnc")
            .expect_err("Did not detect non-function");
        runtime
            .get_function_by_name(&module, "fnd")
            .expect_err("Did not detect undefined");
    }
}
