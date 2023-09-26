use deno_core::anyhow::{anyhow, self};
use deno_core::v8::HandleScope;
use tokio::runtime;
use deno_core::{ 
    serde_json, v8, resolve_path,
    JsRuntime, FsModuleLoader, ModuleSpecifier, ModuleId
};
use std::{
    env::current_dir,
    rc::Rc,
    time::Duration
};

use crate::script::Script;
use crate::error::*;

#[derive(Default)]
/// Represents the set of options accepted by the runtime constructor
pub struct RuntimeOptions {
    /// A set of deno_core extensions to add to the runtime
    pub extensions: Vec<deno_core::Extension>,

    /// Function to use as entrypoint if the script does not provide one
    pub default_entrypoint: Option<String>,

    /// Amount of time to run for before killing the thread
    pub timeout: Option<Duration>
}

mod extensions {
    use deno_core::{ v8, op2, extension, OpState };
    use crate::error::Error;

    #[op2]
    /// Registers a JS function with the runtime as being the entrypoint for the script
        ///
        /// # Arguments
        /// * `state` - The runtime's state, into which the function will be put
        /// * `callback` - The function to register
    fn op_register_entrypoint(state: &mut OpState, #[global] callback: v8::Global<v8::Function>) -> Result<(), Error> {
        state.put(callback);
        Ok(())
    }

    extension!(
        js_playground,
        ops = [op_register_entrypoint],
        esm_entry_point = "ext:js_playground/js_playground.js",
        esm = [ dir "src/ext", "js_playground.js" ],
    );
}

/// Represents a loaded instance of a module within a runtime
pub struct ModuleHandle {
    entrypoint: Option<v8::Global<v8::Function>>,
    module_id: ModuleId,
    module: Script
}

impl ModuleHandle {
    /// Create a new module instance
    pub fn new(module: &Script, module_id: ModuleId, entrypoint: Option<v8::Global<v8::Function>>) -> Self {
        Self {
            module_id, entrypoint, module: module.clone()
        }
    }

    /// Return this module's contents
    pub fn module(&self) -> &Script {
        &self.module
    }

    /// Return this module's ID
    pub fn id(&self) -> &ModuleId {
        &self.module_id
    }

    /// Return this module's entrypoint
    pub fn entrypoint(&self) -> &Option<v8::Global<v8::Function>> {
        &self.entrypoint
    }
}

/// Represents a configured runtime ready to run modules
pub struct Runtime {
    deno_runtime: JsRuntime,
    timeout: Option<Duration>
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
    ///
    /// A `Result` containing either the initialized runtime instance on success (`Ok`) or an error on failure (`Err`).
    ///
    /// # Example
    ///
    /// ```
    /// // Create a new runtime with default options
    /// use js_playground::{ Runtime, RuntimeOptions };
    /// 
    /// let runtime = Runtime::new(RuntimeOptions::default());
    /// match runtime {
    ///     Ok(runtime) => {
    ///         // Successfully created the runtime instance
    ///     }
    ///     Err(error) => {
    ///         // Handle the error
    ///         eprintln!("Failed to create the runtime: {:?}", error);
    ///     }
    /// }
    /// ```
    ///
    pub fn new(options: RuntimeOptions) -> Result<Self, Error> {
        // Prep extensions
        let mut extensions = vec![extensions::js_playground::init_ops_and_esm()];
        extensions.extend(options.extensions);

        let js_runtime = JsRuntime::new(deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(FsModuleLoader)),
            extensions,
            ..Default::default()
        });

        let mut runtime_instance = Self {
            deno_runtime: js_runtime,
            timeout: options.timeout
        };
        
        // Default entrypoint
        if let Some(entrypoint) = options.default_entrypoint {
            if let Ok(function) = runtime_instance.get_function_by_name(None, &entrypoint) {
                let state = runtime_instance.deno_runtime().op_state();
                let mut deep_state = state.try_borrow_mut()?;
                deep_state.put(function);
            }
        }

        Ok(runtime_instance)
    }

    /// Get a mutable borrow of the internal runtime
    pub fn deno_runtime(&mut self) -> &mut JsRuntime {
        &mut self.deno_runtime
    }

    /// resolve a module specifier to the current wd if relative
    fn get_module_specifier(path: &str) -> Result<ModuleSpecifier, Error> {
        Ok(resolve_path(
            path, 
            &current_dir()?
        )?)
    }

    fn get_value_from_v8_object(&mut self, scope: &mut HandleScope, context: v8::Local<v8::Object>, name: &str)-> Result<v8::Global<v8::Value>, Error> {
        // Turn the name into a v8 value
        let key = v8::String::new(scope, name).ok_or(
            V8EncodingError::new(name.to_string())
        )?;

        let value: v8::Local<v8::Value> = context.get(scope, key.into()).ok_or(
            ValueNotFoundError::new(name.to_string())
        )?;
        
        if value.is_undefined() {
            Err(ValueNotFoundError::new(name.to_string()).into())
        } else {
            Ok(v8::Global::<v8::Value>::new(scope, value))
        }
    }

    /// Attempt to get a value out of the global context (globalThis.name)
    fn get_global_value(&mut self, name: &str) -> Result<v8::Global<v8::Value>, Error> {
        let context = self.deno_runtime.main_context();
        let mut scope = self.deno_runtime.handle_scope();
        let global = context.open(&mut scope).global(&mut scope);

        self.get_value_from_v8_object(&mut scope, global, name)
    }
    
    /// Attempt to get a value out of a module context
    fn get_module_export_value(&mut self, module: &ModuleHandle, name: &str) -> Result<v8::Global<v8::Value>, Error> {
        let module_namespace = self.deno_runtime.get_module_namespace(*module.id())?;
        let mut scope = self.deno_runtime.handle_scope();
        let module_namespace = v8::Local::<v8::Object>::new(&mut scope, module_namespace);
        
        self.get_value_from_v8_object(&mut scope, module_namespace, name)
    }

    /// Calls a JavaScript function and deserializes its return value into a Rust type.
    ///
    /// This method takes a JavaScript function and invokes it within the Deno runtime.
    /// It then serializes the return value of the function into a JSON string and
    /// deserializes it into the specified Rust type (`T`).
    ///
    /// # Arguments
    /// * `function` - A reference to a JavaScript function (`v8::Function`)
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function call fails or the return value cannot
    /// be deserialized.
    ///
    /// # Example
    ///
    /// ```
    /// use js_playground::{ Runtime, RuntimeOptions, Script, Error };
    /// 
    /// fn main() -> Result<(), Error> {
    ///     let mut runtime = Runtime::new(Default::default())?;
    ///     let script = Script::new("/path/to/module.ts", "
    ///         const f = () => 2;
    ///     ");
    ///     let module = runtime.load_modules(script, vec![])?;
    ///     let f = runtime.get_function_by_name(Some(&module), "f").unwrap();
    ///     let value: usize = runtime.call_function(f, Runtime::EMPTY_ARGS)?;
    ///     Ok(())
    /// }
    /// ```
    ///
    pub fn call_function<T, A>(&mut self, function: v8::Global<v8::Function>, args: &[A]) -> Result<T, Error>
    where T: deno_core::serde::de::DeserializeOwned, A: deno_core::serde::Serialize {
        let mut scope = self.deno_runtime.handle_scope();
        let function_instance = function.open(&mut scope);
        let name = function_instance.get_name(&mut scope).to_rust_string_lossy(&mut scope);

        // Prep arguments
        let f_args: Result<Vec<v8::Local<v8::Value>>, deno_core::serde_v8::Error>
            = args.iter().map(|f| deno_core::serde_v8::to_v8(&mut scope, f)).collect();
        let final_args = f_args?;

        // Call the function
        let undefined: v8::Local<v8::Value> = v8::undefined(&mut scope).into();
        let result = function_instance.call(&mut scope, undefined, &final_args).ok_or(
            JsonDecodeError::new(anyhow!("{} did not return a value", name))
        )?;

        // Re-Serialize to get a rust value
        let json_string = v8::json::stringify(&mut scope, result).ok_or(
            JsonDecodeError::new(anyhow!("{} returned an invalid value", name))
        )?.to_rust_string_lossy(&mut scope);
        
        Ok(serde_json::from_str(&json_string)?)
    }

    /// Retrieves a JavaScript function by its name from the Deno runtime's global context.
    ///
    /// This method attempts to find a JavaScript function within the global context of the
    /// Deno runtime by specifying its name. If the function is found, it is returned as
    /// a `v8::Global<v8::Function>`, which allows you to safely
    /// reference and use the function.
    ///
    /// # Arguments
    /// * `name` - A string representing the name of the JavaScript function to retrieve.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `v8::Global<v8::Function>` if
    /// the function is found, or an error (`Error`) if the function cannot be found or
    /// if it is not a valid JavaScript function.
    ///
    /// # Example
    ///
    /// ```
    /// use js_playground::{ Runtime, Script, Error };
    /// 
    /// fn main() -> Result<(), Error> {
    ///     let mut runtime = Runtime::new(Default::default())?;
    ///     let script = Script::new("/path/to/module.js", "export function f() { return 2; }");
    ///     let module = runtime.load_modules(script, vec![])?;
    ///     let function = runtime.get_function_by_name(Some(&module), "f")?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// This method is designed to retrieve JavaScript functions by name from the global
    /// context of the Deno runtime. It checks whether the retrieved value is indeed a
    /// JavaScript function and returns it as a global reference.
    ///
    /// Ensure that you have set up the Deno runtime (`Runtime`) correctly before using
    /// this method, and provide the correct function name as an argument.
    ///
    pub fn get_function_by_name(&mut self, module_context: Option<&ModuleHandle>, name: &str) -> Result<v8::Global<v8::Function>, Error> {
        let mut value: Option<v8::Global<v8::Value>> = None;

        // Get the value (try module first)
        if let Some(module) = module_context {
            if let Ok(v) = self.get_module_export_value(module, name) {
                value = Some(v);
            }
        }
        
        // Get the value (then globals)
        if let Ok(v) = self.get_global_value(name) {
            value = Some(v);
        }

        let mut scope = self.deno_runtime.handle_scope();
        if value.is_none() || value.clone().unwrap().open(&mut scope).is_undefined() {
            Err(ValueNotFoundError::new(name.to_string()).into())
        } else {

            // Need to turn it back into a local for casting
            let local_value = v8::Local::<v8::Value>::new(&mut scope, value.unwrap());
            println!("{:?}", local_value);

            // Convert it into a function
            let f: v8::Local<v8::Function> = local_value.try_into().or::<Error>(
                Err(ValueNotCallableError::new(name.to_string()).into())
            )?;
            
            // Return it as a global
            Ok(v8::Global::<v8::Function>::new(&mut scope, f))
        }
    }

    /// Calls a JavaScript function within the Deno runtime by its name and deserializes its return value.
    ///
    /// This method allows you to call a JavaScript function within the Deno runtime by specifying
    /// its name. It retrieves the function using the `get_function_by_name` method and then invokes
    /// it. Finally, it deserializes the return value of the function into the specified Rust type (`T`).
    ///
    /// # Arguments
    ///
    /// * `name` - A string representing the name of the JavaScript function to call.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function cannot be found, if there are issues with
    /// calling the function, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::{ Runtime, Script, Error };
    /// 
    /// fn main() -> Result<(), Error> {
    ///     let mut runtime = Runtime::new(Default::default())?;
    ///     let script = Script::new("/path/to/module.js", "export function f() { return 2; };");
    ///     let module = runtime.load_modules(script, vec![])?;
    ///     let value: usize = runtime.call_function_by_name(Some(&module), "f", Runtime::EMPTY_ARGS)?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// This method simplifies the process of calling a JavaScript function within the Deno runtime
    /// by its name. It internally uses the `get_function_by_name` method to retrieve the function
    /// and then invokes it using the `call_function` method. Ensure that you have set up the Deno
    /// runtime (`Runtime`) correctly and provide the correct function name as an argument.
    ///
    pub fn call_function_by_name<T, A>(&mut self, module_context: Option<&ModuleHandle>, name: &str, args: &[A]) -> Result<T, Error>
    where T: deno_core::serde::de::DeserializeOwned, A: deno_core::serde::Serialize {
        let function = self.get_function_by_name(module_context, name)?;
        self.call_function(function, args)
    }

    /// Runs a JavaScript script within the Deno runtime and returns its result.
    ///
    /// This method allows you to execute a JavaScript script within the Deno runtime.
    /// It takes a `Script` object representing the script to run and returns the
    /// result of the script's execution, deserialized into the specified Rust type (`T`).
    ///
    /// # Arguments
    /// * `module` - A `Script` object containing the module's filename and contents.
    /// * `side_modules` - A set of additional modules to be loaded into memory for use
    ///
    /// # Returns
    ///
    /// A `Result` containing the ID for the loaded module
    /// or an error (`Error`) if there are issues with loading modules, executing the
    /// script, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create a script with filename and contents
    /// use js_playground::{Runtime, Script, Error};
    /// 
    /// fn main() -> Result<(), Error> {
    ///     // Create a Deno runtime and a script
    ///     let mut runtime = Runtime::new(Default::default())?;
    ///     let script = Script::new("test.js", "js_playground.register_entrypoint(() => 'test')");
    ///     runtime.load_modules(script, vec![]);
    ///     Ok(())
    /// }
    /// ```
    ///
    #[allow(unused_assignments)]
    pub fn load_modules(&mut self, module: Script, side_modules: Vec<Script>) -> Result<ModuleHandle, Error> {

        // Evaluate the script
        let deno_runtime = &mut self.deno_runtime;
        let mut module_id: ModuleId = 0;
        let mod_copy = module.clone();
        let future = async move {
            // Get additional modules
            for side_module in side_modules {
                let s_modid = deno_runtime.load_side_module(
                    &Self::get_module_specifier(side_module.filename())?, 
                    Some(deno_core::FastString::from(side_module.contents().to_string()))
                ).await?;
                deno_runtime.mod_evaluate(s_modid);
            }

            // Load main module
            module_id = deno_runtime.load_main_module(
                &Self::get_module_specifier(module.filename())?, 
                Some(deno_core::FastString::from(module.contents().to_string()))
            ).await?;

            // Finish execution
            let result = deno_runtime.mod_evaluate(module_id);
            deno_runtime.run_event_loop(false).await?;
            result.await?
        };
        
        // Get the thread ready
        let tokio_runtime = runtime::Builder::new_current_thread()
            .enable_all().build()?;

        // Let it play out...
        tokio_runtime.block_on(future)?;
        if let Some(timeout) = self.timeout {
            tokio_runtime.shutdown_timeout(timeout);
        }

        // Try to get an entrypoint
        let state = self.deno_runtime.op_state();
        let mut deep_state = state.try_borrow_mut()?;
        Ok(ModuleHandle::new(
            &mod_copy,
            module_id,
            deep_state.try_take::<v8::Global<v8::Function>>()
        ))
    }
    /// Executes the entrypoint function of a script within the Deno runtime.
    ///
    /// This method attempts to retrieve and execute the entrypoint function within the Deno runtime.
    /// If the entrypoint function is found, it is invoked, and the result is deserialized into
    /// the specified Rust type (`T`).
    ///
    /// # Arguments
    /// * `module_context` - A context object returned by loading a module into the runtime
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized result of the entrypoint execution (`T`)
    /// if successful, or an error (`Error`) if the entrypoint is missing, the execution fails,
    /// or the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use js_playground::{Runtime, Script, Error};
    /// 
    /// fn main() -> Result<(), Error> {
    ///     // Create a Deno runtime and a script
    ///     let mut runtime = Runtime::new(Default::default())?;
    ///     let script = Script::new("test.js", "js_playground.register_entrypoint(() => 'test')");
    ///     let module = runtime.load_modules(script, vec![]);
    ///
    ///     // Run the entrypoint and handle the result
    ///     let value: String = runtime.call_entrypoint(&module)?;
    ///     Ok(())
    /// }
    /// ```
    ///
    pub fn call_entrypoint<T>(&mut self, module_context: &ModuleHandle) -> Result<T, Error>
    where T: deno_core::serde::de::DeserializeOwned {
        if let Some(entrypoint) = &module_context.entrypoint {
            let value: serde_json::Value = self.call_function(entrypoint.clone(), Self::EMPTY_ARGS)?;
            Ok(serde_json::from_value(value)?)
        } else {
            Err(MissingEntrypointError::new(module_context.module().clone()).into())
        }
    }

    /// Loads a module into a new runtime, executes the entry function and returns the
    /// result of the script's execution, deserialized into the specified Rust type (`T`).
    ///
    /// # Arguments
    /// * `module` - A `Script` object containing the module's filename and contents.
    /// * `side_modules` - A set of additional modules to be loaded into memory for use
    /// * `runtime_options` - Options for the creation of the runtime
    ///
    /// # Returns
    ///
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
    /// fn main() -> Result<(), Error> {
    ///     let script = Script::new("test.js", "js_playground.register_entrypoint(() => 2)");
    ///     let value: usize = Runtime::execute_module(script, vec![], Default::default())?;
    ///     Ok(())
    /// }
    /// ```
    ///
    pub fn execute_module<T>(module: Script, side_modules: Vec<Script>, runtime_options: RuntimeOptions) -> Result<T, Error>
    where T: deno_core::serde::de::DeserializeOwned {
        let mut runtime = Runtime::new(runtime_options)?;
        let module = runtime.load_modules(module, side_modules)?;
        let value: T = runtime.call_entrypoint(&module)?;
        Ok(value)
    }
}

#[cfg(test)]
mod test_runtime {
    use super::*;
    
    #[test]
    fn test_run() {
        let script = Script::new("test.js", "
            js_playground.register_entrypoint(function() {
                console.log('Hello World');
                return 2;
            })
        ");
        let mut runtime = Runtime::new(Default::default()).unwrap();
        let module = runtime.load_modules(script, vec![]).unwrap();

        let value: serde_json::Value = runtime.call_entrypoint(&module).unwrap();
        assert_eq!(value, 2);
    }

    #[test]
    fn test_get_module_export_value() {
        let script = Script::new("test.js", "
            export const test_value = 1;
            export function test_func() {
                return 'test';
            }
        ");
        let mut runtime = Runtime::new(Default::default()).unwrap();
        let module = runtime.load_modules(script, vec![]).unwrap();

        let v1 = runtime.get_module_export_value(&module, "test_value").unwrap();
        let v1_local = v8::Local::<v8::Value>::new(&mut runtime.deno_runtime.handle_scope(), v1);
        assert!(!v1_local.is_undefined());
        assert!(!v1_local.is_number());

        let v1 = runtime.get_module_export_value(&module, "test_func").unwrap();
        let v1_local = v8::Local::<v8::Value>::new(&mut runtime.deno_runtime.handle_scope(), v1);
        assert!(!v1_local.is_undefined());
        assert!(!v1_local.is_function());
    }
}