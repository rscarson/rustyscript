use crate::{
    inner_runtime::{InnerRuntime, InnerRuntimeOptions, RsAsyncFunction, RsFunction, AsyncInnerRuntime},
    Error, FunctionArguments, JsFunction, Module, ModuleHandle,
};
use deno_core::serde_json;

/// Represents the set of options accepted by the runtime constructor
pub type AsyncRuntimeOptions = InnerRuntimeOptions;
pub type RuntimeOptions = AsyncRuntimeOptions;

/// For functions returning nothing
pub type Undefined = serde_json::Value;

/// Represents a configured runtime ready to run modules
pub struct AsyncRuntime(InnerRuntime);

impl AsyncRuntime {
    /// The lack of any arguments - used to simplify calling functions
    /// Prevents you from needing to specify the type using ::<serde_json::Value>
    pub const EMPTY_ARGS: &'static FunctionArguments = &[];

    /// Creates a new instance of the runtime with the provided options.
    ///
    /// The async runtime must run in a single threaded Tokio runtime
    ///
    /// # Arguments
    /// * `options` - A `RuntimeOptions` struct that specifies the configuration options for the runtime.
    ///
    /// # Returns
    /// A `Result` containing either the initialized runtime instance on success (`Ok`) or an error on failure (`Err`).
    ///
    pub fn new(options: RuntimeOptions) -> Result<Self, Error> {
        Ok(Self(InnerRuntime::new(options)?))
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
    pub fn arg<A>(value: A) -> Result<serde_json::Value, Error>
    where
        A: serde::Serialize,
    {
        Ok(serde_json::to_value(value)?)
    }

    /// Encode a primitive as a json value for use as a function argument
    /// Only for types with `Into<Value>`. For other types, use `Runtime::arg`
    pub fn into_arg<A>(value: A) -> serde_json::Value
    where
        serde_json::Value: From<A>,
    {
        serde_json::Value::from(value)
    }

    /// Remove and return a value from the state, if one exists
    pub fn take<T>(&mut self) -> Option<T>
    where
        T: 'static,
    {
        self.0.take()
    }

    /// Add a value to the state
    /// Only one value of each type is stored - additional calls to put overwrite the
    /// old value
    /// ```rust
    /// use rustyscript::{ Runtime };
    ///
    /// # fn main() -> Result<(), rustyscript::Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// runtime.put("test".to_string())?;
    /// let value: String = runtime.take().unwrap();
    /// assert_eq!(value, "test");
    /// # Ok(())
    /// # }
    /// ```
    pub fn put<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: 'static,
    {
        self.0.put(value)
    }

    /// Register a rust function to be callable from JS
    pub fn register_function<F>(&mut self, name: &str, callback: F) -> Result<(), Error>
    where
        F: RsFunction,
    {
        self.0.register_function(name, callback)
    }

    /// Register a non-blocking rust function to be callable from JS
    pub fn register_async_function<F>(&mut self, name: &str, callback: F) -> Result<(), Error>
    where
        F: RsAsyncFunction,
    {
        self.0.register_async_function(name, callback)
    }

    /// Evaluate a piece of non-ECMAScript-module JavaScript code
    /// The expression is evaluated in the global context, so changes persist
    ///
    /// # Arguments
    /// * `expr` - A string representing the JavaScript expression to evaluate
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the expression (`T`)
    /// or an error (`Error`) if the expression cannot be evaluated or if the
    /// result cannot be deserialized.
    pub fn eval<T>(&mut self, expr: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.0.eval(expr)
    }

    /// Calls a stored javascript function and deserializes its return value.
    ///
    /// # Arguments
    /// * `module_context` - Optional handle to a module providing global context for the function
    /// * `function` - A The function object
    /// * `args` - The arguments to pass to the function
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function cannot be found, if there are issues with
    /// calling the function, or if the result cannot be deserialized.
    pub async fn call_stored_function<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        function: &JsFunction<'_>,
        args: &FunctionArguments,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        self.0.call_stored_function(module_context, function, args).await
    }

    /// Calls a javascript function within the Deno runtime by its name and deserializes its return value.
    ///
    /// # Arguments
    /// * `module_context` - Optional handle to a module to search - if None, or if the search fails, the global context is used
    /// * `name` - A string representing the name of the javascript function to call.
    /// * `args` - The arguments to pass to the function
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function cannot be found, if there are issues with
    /// calling the function, or if the result cannot be deserialized.
    pub async fn call_function<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
        args: &FunctionArguments,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        self.0.call_function(module_context, name, args).await
    }

    /// Get a value from a runtime instance
    ///
    /// # Arguments
    /// * `module_context` - Optional handle to a module to search - if None, or if the search fails, the global context is used
    /// * `name` - A string representing the name of the value to find
    ///
    /// # Returns
    /// A `Result` containing the deserialized result or an error (`Error`) if the
    /// value cannot be found, if there are issues with, or if the result cannot be
    ///  deserialized.
    pub async fn get_value<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.0.get_value(module_context, name).await
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
    pub async fn load_module(&mut self, module: &Module) -> Result<ModuleHandle, Error> {
        self.0.load_modules(None, vec![module]).await
    }

    /// Executes the given module, and returns a handle allowing you to extract values
    /// And call functions.
    ///
    /// This will load 'module' as the main module, and the others as side-modules.
    /// Only one main module can be loaded per runtime
    ///
    /// # Arguments
    /// * `module` - A `Module` object containing the module's filename and contents.
    /// * `side_modules` - A set of additional modules to be loaded into memory for use
    ///
    /// # Returns
    /// A `Result` containing a handle for the loaded module
    /// or an error (`Error`) if there are issues with loading modules, executing the
    /// module, or if the result cannot be deserialized.
    pub async fn load_modules(
        &mut self,
        module: &Module,
        side_modules: Vec<&Module>,
    ) -> Result<ModuleHandle, Error> {
        self.0.load_modules(Some(module), side_modules).await
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
    pub async fn call_entrypoint<T>(
        &mut self,
        module_context: &ModuleHandle,
        args: &FunctionArguments,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        if let Some(entrypoint) = module_context.entrypoint() {
            let value: serde_json::Value = self.0.call_function_by_ref(
                Some(module_context),
                entrypoint.clone(),
                args,
            ).await?;
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
    pub async fn execute_module<T>(
        module: &Module,
        side_modules: Vec<&Module>,
        runtime_options: RuntimeOptions,
        entrypoint_args: &FunctionArguments,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        let mut runtime = Self::new(runtime_options)?;
        let module = runtime.load_modules(module, side_modules).await?;
        let value: T = runtime.call_entrypoint(&module, entrypoint_args).await?;
        Ok(value)
    }
}

// TODO: test
