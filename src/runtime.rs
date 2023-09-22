use deno_core::{ JsRuntime, FsModuleLoader, resolve_path, v8, OpState , op2, extension, ModuleSpecifier};
use tokio::runtime;
use std::env::current_dir;
use std::rc::Rc;
use std::time::Duration;

use crate::script::Script;
use crate::error::*;

#[derive(Default)]
pub struct RuntimeOptions {
    pub extensions: Vec<deno_core::Extension>,
    pub default_entrypoint: Option<String>,
    pub timeout: Option<Duration>
}

/// Represents a configured runtime ready to run modules
pub struct Runtime {
    deno_runtime: JsRuntime,
    modules: Vec<Script>,
    timeout: Option<Duration>
}

#[op2]
pub fn op_register_entrypoint(state: &mut OpState, #[global] callback: v8::Global<v8::Function>) -> Result<(), Error> {
    state.put(callback);
    Ok(())
}
extension!(
    js_playground,
    ops = [op_register_entrypoint],
    esm_entry_point = "ext:js_playground/js_playground.js",
    esm = [ dir "src/ext", "js_playground.js" ],
);

impl Runtime {
    const EMTPY_ARGS: &[deno_core::serde_json::Value] = &[];

    pub fn new(options: RuntimeOptions) -> Result<Self, Error> {
        // Prep extensions
        let mut extensions = vec![js_playground::init_ops_and_esm()];
        extensions.extend(options.extensions);

        let js_runtime = JsRuntime::new(deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(FsModuleLoader)),
            extensions,
            ..Default::default()
        });

        let mut runtime_instance = Self {
            deno_runtime: js_runtime,
            modules: Vec::new(),
            timeout: options.timeout
        };
        
        // Default entrypoint
        if let Some(entrypoint) = options.default_entrypoint {
            let state = runtime_instance.deno_runtime().op_state();
            let mut deep_state = state.try_borrow_mut()?;
            deep_state.put(
                runtime_instance.get_function_by_name(&entrypoint)
            );
        }

        Ok(runtime_instance)
    }

    /// Get a mutable borrow of the internal runtime
    pub fn deno_runtime(&mut self) -> &mut JsRuntime {
        &mut self.deno_runtime
    }

    /// Get the set of loaded side-modules
    pub fn modules(&self) -> &Vec<Script> {
        &self.modules
    }

    ///
    fn get_module_specifier(path: &str) -> Result<ModuleSpecifier, Error> {
        Ok(resolve_path(
            path, 
            &current_dir()?
        )?)
    }

    /// Inserts a new module into the internal collection of modules.
    ///
    /// # Arguments
    /// * `specifier` - An absolute path specifying the location of the module.
    /// * `code` - The source code content of the module as a string.
    ///
    /// # Example
    ///
    /// ```
    /// let mut runtime = Runtime::new(Default::default());
    /// let script = Script::new("/path/to/script.js", "console.log('Hello, Deno!');");
    /// runtime.add_module(script);
    /// ```
    ///
    pub fn add_module(&mut self, script: Script) {
        self.modules.push(script);
    }

    /// Calls a JavaScript function and deserializes its return value into a Rust type.
    ///
    /// This method takes a JavaScript function and invokes it within the Deno runtime.
    /// It then serializes the return value of the function into a JSON string and
    /// deserializes it into the specified Rust type (`T`).
    ///
    /// # Arguments
    /// * `function` - A reference to a JavaScript function (`deno_core::v8::Function`)
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
    /// let mut runtime = Runtime::new(Default::default());
    /// runtime.run_module("/path/to/module.ts", "function f() { return 2; }");
    /// let f = runtime.get_function_by_name('f').unwrap();
    /// let value: usize = runtime.call_function(f);
    /// ```
    ///
    /// # Note
    ///
    /// This method is designed to call JavaScript functions within the Deno runtime.
    /// It assumes that the function being called returns a JSON-serializable value.
    /// If the JavaScript function does not return a valid JSON-serializable value,
    /// or if there are errors during the function call, this method will return an error.
    ///
    /// Ensure that you have set up the Deno runtime (`deno_runtime`) and imported
    /// the necessary modules correctly before using this method.
    ///
    pub fn call_function<T, A>(&mut self, function: deno_core::v8::Global<deno_core::v8::Function>, args: &[A]) -> Result<T, Error>
    where T: deno_core::serde::de::DeserializeOwned, A: deno_core::serde::Serialize {
        let mut scope = self.deno_runtime.handle_scope();
        let function_instance = function.open(&mut scope);
        let name = function_instance.get_name(&mut scope).to_rust_string_lossy(&mut scope);

        // Prep arguments
        let f_args: Result<Vec<v8::Local<v8::Value>>, deno_core::serde_v8::Error>
            = args.iter().map(|f| deno_core::serde_v8::to_v8(&mut scope, f)).collect();
        let final_args = f_args?;

        // Call the function
        let undefined: deno_core::v8::Local<deno_core::v8::Value> = deno_core::v8::undefined(&mut scope).into();
        let result = function_instance.call(&mut scope, undefined, &final_args).ok_or(
            JsonDecodeError::new_from_string(&format!("{} did not return a value", name))
        )?;

        // Re-Serialize to get a rust value
        let json_string = deno_core::v8::json::stringify(&mut scope, result).ok_or(
            JsonDecodeError::new_from_string(&format!("{} returned an invalid value", name))
        )?.to_rust_string_lossy(&mut scope);
        
        Ok(deno_core::serde_json::from_str(&json_string)?)
    }

    /// Retrieves a JavaScript function by its name from the Deno runtime's global context.
    ///
    /// This method attempts to find a JavaScript function within the global context of the
    /// Deno runtime by specifying its name. If the function is found, it is returned as
    /// a `deno_core::v8::Global<deno_core::v8::Function>`, which allows you to safely
    /// reference and use the function.
    ///
    /// # Arguments
    /// * `name` - A string representing the name of the JavaScript function to retrieve.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `deno_core::v8::Global<deno_core::v8::Function>` if
    /// the function is found, or an error (`Error`) if the function cannot be found or
    /// if it is not a valid JavaScript function.
    ///
    /// # Example
    ///
    /// ```
    /// let mut runtime = Runtime::new(Default::default());
    /// runtime.run_module("/path/to/module.ts", "function f() { return 2; }");
    /// match runtime.get_function_by_name("f") {
    ///     Ok(function) => {
    ///         // Use the retrieved function here
    ///         println!("Found function: {:?}", function);
    ///     }
    ///     Err(error) => {
    ///         // Handle the error here
    ///         eprintln!("Error finding function: {}", error);
    ///     }
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
    pub fn get_function_by_name(&mut self, name: &str) -> Result<deno_core::v8::Global<deno_core::v8::Function>, Error> {
        let context = self.deno_runtime.main_context();
        let mut scope = self.deno_runtime.handle_scope();
        let global = context.open(&mut scope).global(&mut scope);

        let func_key = deno_core::v8::String::new(&mut scope, name).unwrap();
        let func: deno_core::v8::Local<deno_core::v8::Function> = global.get(&mut scope, func_key.into()).ok_or(
            RuntimeError::new_from_string(&format!("{} was not found in the global scope", name))
        )?.try_into()?;
        
        if func.is_function() {
            Ok(deno_core::v8::Global::new(&mut scope, func))
        } else {
            Err(RuntimeError::new_from_string(&format!("{} is not a function", name)).into())
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
    /// let mut runtime = Runtime::new(Default::default())
    /// runtime.run_module("/path/to/module.ts", "function f() { return 2; }");
    ///
    /// // Call the function by name and handle the result
    /// match runtime.call_function_by_name::<usize>("f") {
    ///     Ok(result) => {
    ///         // Handle the deserialized result here
    ///         println!("Function result: {:?}", result);
    ///     }
    ///     Err(error) => {
    ///         // Handle the error here
    ///         eprintln!("Error calling function: {}", error);
    ///     }
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
    pub fn call_function_by_name<T, A>(&mut self, name: &str, args: &[A]) -> Result<T, Error>
    where T: deno_core::serde::de::DeserializeOwned, A: deno_core::serde::Serialize {
        let function = self.get_function_by_name(name)?;
        self.call_function(function, args)
    }

    /// Runs a JavaScript script within the Deno runtime and returns its result.
    ///
    /// This method allows you to execute a JavaScript script within the Deno runtime.
    /// It takes a `Script` object representing the script to run and returns the
    /// result of the script's execution, deserialized into the specified Rust type (`T`).
    ///
    /// # Arguments
    /// * `script` - A `Script` object containing the script's filename and contents.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized result of the script execution (`T`)
    /// or an error (`Error`) if there are issues with loading modules, executing the
    /// script, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create a script with filename and contents
    /// let script = Script::new("/path/to/script.js", "console.log('Hello, Deno!');");
    ///
    /// // Create a Deno runtime
    /// let mut runtime = Runtime::new(Default::default());
    ///
    /// // Run the script and handle the result
    /// match runtime.run_module::<String>(script) {
    ///     Ok(result) => {
    ///         // Handle the deserialized result here
    ///         println!("Script result: {:?}", result);
    ///     }
    ///     Err(error) => {
    ///         // Handle the error here
    ///         eprintln!("Error running script: {}", error);
    ///     }
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// This method loads any required modules specified within the script and then
    /// evaluates the main script. It expects that the result of the script execution
    /// is a JSON-serializable value. If the script execution fails, or if the result
    /// cannot be deserialized, an error will be returned.
    ///
    /// Paths should be absolute, and ES modules are expected
    ///
    pub fn run_module<T>(&mut self, script: Script) -> Result<T, Error>
    where T: deno_core::serde::de::DeserializeOwned {

        // Evaluate the script
        let modules = &self.modules;
        let deno_runtime = &mut self.deno_runtime;
        let future = async move {
            // Get additional modules
            for side_script in modules {
                deno_runtime.load_side_module(
                    &Self::get_module_specifier(side_script.filename())?, 
                    Some(deno_core::FastString::from(side_script.contents().to_string()))
                ).await?;
            }

            // Load main module
            let mod_id = deno_runtime.load_main_module(
                &Self::get_module_specifier(script.filename())?, 
                Some(deno_core::FastString::from(script.contents().to_string()))
            ).await?;

            // Finish execution
            let result = deno_runtime.mod_evaluate(mod_id);
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

        // If we have an entrypoint, use it
        let state = self.deno_runtime.op_state();
        let mut deep_state = state.try_borrow_mut()?;
        if let Some(entrypoint) = deep_state.try_take::<v8::Global<v8::Function>>() {
            let value: deno_core::serde_json::Value = self.call_function(entrypoint, Self::EMTPY_ARGS)?;
            Ok(deno_core::serde_json::from_value(value)?)
        } else {
            Err(MissingEntrypointError::new_from_string(
                "Entrypoint was no specified. Provide a default for this runtime, or ensure scripts call js_playground.register_entrypoint()"
            ).into())
        }
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
        let _value: usize = runtime.run_module(script).unwrap();
        assert_eq!(_value, 2);
    }
}