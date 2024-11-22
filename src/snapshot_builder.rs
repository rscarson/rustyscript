use crate::{
    async_bridge::{AsyncBridge, AsyncBridgeExt},
    inner_runtime::{InnerRuntime, RuntimeOptions},
    Error, Module, ModuleHandle,
};
use deno_core::{JsRuntimeForSnapshot, PollEventLoopOptions};
use std::{path::Path, rc::Rc, time::Duration};
use tokio_util::sync::CancellationToken;

/// A more restricted version of the `Runtime` struct that is used to create a snapshot of the runtime state
/// This runtime should ONLY be used to create a snapshot, and not for normal use
///
/// Snapshots can be used to massively decrease the startup time of a Runtime instance (15ms -> 3ms) by pre-loading
/// extensions and modules into the runtime state before it is created. A snapshot can be used on any runtime with
/// the same set of extensions and options as the runtime that created it.
///
/// This struct is only available when the `snapshot_builder` feature is enabled
/// Once you've set up the runtime, you can call `into_snapshot` to get the snapshot
///
/// You should save it to a file and load it with `include_bytes!` in order to use it
/// in the `RuntimeOptions` struct's `startup_snapshot` field
///
/// # Example
///
/// ```rust
/// use rustyscript::{SnapshotBuilder, Module, Error};
/// use std::fs;
///
/// # fn main() -> Result<(), Error> {
/// let module = Module::new("example.js", "export function example() { return 42; }");
/// let snapshot = SnapshotBuilder::new(Default::default())?
///    .with_module(&module)?
///    .finish();
///
/// // Save the snapshot to a file
/// fs::write("snapshot.bin", snapshot)?;
///
/// // To use the snapshot, load it with `include_bytes!` into the `RuntimeOptions` struct:
/// // const STARTUP_SNAPSHOT: &[u8] = include_bytes!("snapshot.bin");
/// // RuntimeOptions {
/// //     startup_snapshot: Some(STARTUP_SNAPSHOT),
/// //     ..Default::default()
/// // };
///
/// # Ok(())
/// # }
/// ```
pub struct SnapshotBuilder {
    inner: InnerRuntime<deno_core::JsRuntimeForSnapshot>,
    tokio: AsyncBridge,
}
impl SnapshotBuilder {
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
    /// use rustyscript::{ json_args, Runtime, RuntimeOptions, Module };
    /// use std::time::Duration;
    ///
    /// # fn main() -> Result<(), rustyscript::Error> {
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
    /// # Errors
    /// Can fail if the tokio runtime cannot be created,
    /// Or if the deno runtime initialization fails (usually issues with extensions)
    ///
    pub fn new(options: RuntimeOptions) -> Result<Self, Error> {
        let tokio = AsyncBridge::new(options.timeout)?;
        let inner = InnerRuntime::new(options, tokio.heap_exhausted_token())?;
        Ok(Self { inner, tokio })
    }

    /// Creates a new instance of the runtime with the provided options and a pre-configured tokio runtime.
    /// See [`crate::Runtime::new`] for more information.
    ///
    /// # Errors
    /// Can fail if the deno runtime initialization fails (usually issues with extensions)
    pub fn with_tokio_runtime(
        options: RuntimeOptions,
        tokio: Rc<tokio::runtime::Runtime>,
    ) -> Result<Self, Error> {
        let tokio = AsyncBridge::with_tokio_runtime(options.timeout, tokio);
        let inner = InnerRuntime::new(options, tokio.heap_exhausted_token())?;
        Ok(Self { inner, tokio })
    }

    /// Access the underlying deno runtime instance directly
    pub fn deno_runtime(&mut self) -> &mut deno_core::JsRuntime {
        self.inner.deno_runtime()
    }

    /// Access the underlying tokio runtime used for blocking operations
    #[must_use]
    pub fn tokio_runtime(&self) -> std::rc::Rc<tokio::runtime::Runtime> {
        self.tokio.tokio_runtime()
    }

    /// Returns the timeout for the runtime
    #[must_use]
    pub fn timeout(&self) -> std::time::Duration {
        self.tokio.timeout()
    }

    /// Returns the heap exhausted token for the runtime
    /// Used to detect when the runtime has run out of memory
    #[must_use]
    pub fn heap_exhausted_token(&self) -> CancellationToken {
        self.tokio.heap_exhausted_token()
    }

    /// Destroy the v8 runtime, releasing all resources
    /// Then the internal tokio runtime will be returned
    #[must_use]
    pub fn into_tokio_runtime(self) -> Rc<tokio::runtime::Runtime> {
        self.tokio.into_tokio_runtime()
    }

    /// Set the current working directory for the runtime
    /// This is used to resolve relative paths in the module loader
    ///
    /// The runtime will begin with the current working directory of the process
    ///
    /// # Errors
    /// Can fail if the given path is not valid
    pub fn set_current_dir(&mut self, path: impl AsRef<Path>) -> Result<&Path, Error> {
        self.inner.set_current_dir(path)
    }

    /// Get the current working directory for the runtime
    /// This is used to resolve relative paths in the module loader
    ///
    /// The runtime will begin with the current working directory of the process
    #[must_use]
    pub fn current_dir(&self) -> &Path {
        self.inner.current_dir()
    }

    /// Advance the JS event loop by a single tick
    /// See [`crate::Runtime::await_event_loop`] for fully running the event loop
    ///
    /// Returns true if the event loop has pending work, or false if it has completed
    ///
    /// # Arguments
    /// * `options` - Options for the event loop polling, see [`deno_core::PollEventLoopOptions`]
    ///
    /// # Errors
    /// Can fail if a runtime error occurs during the event loop's execution
    pub fn advance_event_loop(&mut self, options: PollEventLoopOptions) -> Result<bool, Error> {
        self.block_on(|runtime| async move { runtime.inner.advance_event_loop(options).await })
    }

    /// Run the JS event loop to completion, or until a timeout is reached
    /// Required when using the `_immediate` variants of functions
    ///
    /// # Arguments
    /// * `options` - Options for the event loop polling, see [`deno_core::PollEventLoopOptions`]
    /// * `timeout` - Optional timeout for the event loop
    ///
    /// # Errors
    /// Can fail if a runtime error occurs during the event loop's execution
    pub async fn await_event_loop(
        &mut self,
        options: PollEventLoopOptions,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        self.inner.await_event_loop(options, timeout).await
    }

    /// Run the JS event loop to completion, or until a timeout is reached
    /// Required when using the `_immediate` variants of functions
    ///
    /// This is the blocking variant of [`crate::Runtime::await_event_loop`]
    ///
    /// # Arguments
    /// * `options` - Options for the event loop polling, see [`deno_core::PollEventLoopOptions`]
    /// * `timeout` - Optional timeout for the event loop
    ///
    /// # Errors
    /// Can fail if a runtime error occurs during the event loop's execution
    pub fn block_on_event_loop(
        &mut self,
        options: deno_core::PollEventLoopOptions,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        self.block_on(|runtime| async move { runtime.await_event_loop(options, timeout).await })
    }

    /// Remove and return a value from the state, if one exists
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
    pub fn take<T>(&mut self) -> Option<T>
    where
        T: 'static,
    {
        self.inner.take()
    }

    /// Add a value to the state
    /// Only one value of each type is stored - additional calls to put overwrite the
    /// old value
    ///
    /// # Errors
    /// Can fail if the inner state cannot be borrowed mutably
    ///
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
        self.inner.put(value)
    }

    /// Evaluate a piece of non-ECMAScript-module JavaScript code
    /// The expression is evaluated in the global context, so changes persist
    ///
    /// Asynchronous code is supported, partially
    /// - Top-level await is not supported
    /// - The event loop will be run to completion after the expression is evaluated
    /// - Eval must be run inside a tokio runtime for some async operations
    ///
    /// For proper async support, use one of:
    /// - `call_function_async`
    /// - `call_stored_function_async`
    /// - `load_module_async`
    /// - `load_modules_async`
    ///
    /// Or any of the `_immmediate` variants, paired with [`crate::js_value::Promise`]
    ///
    /// # Arguments
    /// * `expr` - A string representing the JavaScript expression to evaluate
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the expression (`T`)
    /// or an error (`Error`) if the expression cannot be evaluated or if the
    /// result cannot be deserialized.
    ///
    /// # Errors
    /// Can fail if the expression cannot be evaluated, or if the result cannot be deserialized into the requested type
    ///
    /// # Example
    /// ```rust
    /// use rustyscript::{ Runtime, Error };
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let value:
    ///    usize = runtime.eval("2 + 2")?;
    /// assert_eq!(4, value);
    /// # Ok(())
    /// # }
    /// ```
    pub fn eval<T>(&mut self, expr: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.inner.eval(expr)
    }

    /// Calls a javascript function within the Deno runtime by its name and deserializes its return value.
    /// Returns a future that resolves when:
    /// - The event loop is resolved, and
    /// - If the value is a promise, the promise is resolved
    ///
    /// Note that synchronous functions are run synchronously. Returned promises will be run asynchronously, however.
    ///
    /// See [`crate::Runtime::call_function`] for an example
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
    ///
    /// # Errors
    /// Fails if the function cannot be found, if there are issues with calling the function,
    /// Or if the result cannot be deserialized into the requested type
    pub async fn call_function_async<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
        args: &impl serde::ser::Serialize,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        let function = self.inner.get_function_by_name(module_context, name)?;
        let result = self
            .inner
            .call_function_by_ref(module_context, &function, args)?;
        let result = self.inner.resolve_with_event_loop(result).await?;
        self.inner.decode_value(result)
    }

    /// Calls a javascript function within the Deno runtime by its name and deserializes its return value.
    /// Blocks until:
    /// - The event loop is resolved, and
    /// - If the value is a promise, the promise is resolved
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
    ///
    /// # Errors
    /// Fails if the function cannot be found, if there are issues with calling the function,
    /// Or if the result cannot be deserialized into the requested type
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::{ json_args, Runtime, Module, Error };
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("/path/to/module.js", "export function f() { return 2; };");
    /// let module = runtime.load_module(&module)?;
    /// let value: usize = runtime.call_function(Some(&module), "f", json_args!())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn call_function<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
        args: &impl serde::ser::Serialize,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        self.block_on(|runtime| async move {
            runtime
                .call_function_async(module_context, name, args)
                .await
        })
    }

    /// Calls a javascript function within the Deno runtime by its name and deserializes its return value.
    /// Will not attempt to resolve promises, or run the event loop
    /// Promises can be returned by specifying the return type as [`crate::js_value::Promise`]
    /// The event loop should be run using [`crate::Runtime::await_event_loop`]
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
    ///
    /// # Errors
    /// Fails if the function cannot be found, if there are issues with calling the function,
    /// Or if the result cannot be deserialized into the requested type
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::{ json_args, Runtime, Module, Error };
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("/path/to/module.js", "export function f() { return 2; };");
    /// let module = runtime.load_module(&module)?;
    /// let value: usize = runtime.call_function_immediate(Some(&module), "f", json_args!())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn call_function_immediate<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
        args: &impl serde::ser::Serialize,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        let function = self.inner.get_function_by_name(module_context, name)?;
        let result = self
            .inner
            .call_function_by_ref(module_context, &function, args)?;
        self.inner.decode_value(result)
    }

    /// Get a value from a runtime instance
    /// Blocks until:
    /// - The event loop is resolved, and
    /// - If the value is a promise, the promise is resolved
    ///
    /// # Arguments
    /// * `module_context` - Optional handle to a module to search - if None, or if the search fails, the global context is used
    /// * `name` - A string representing the name of the value to find
    ///
    /// # Returns
    /// A `Result` containing the deserialized result or an error (`Error`) if the value cannot be found,
    /// Or if the result cannot be deserialized into the requested type
    ///
    /// # Errors
    /// Can fail if the value cannot be found, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::{ Runtime, Module, Error };
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("/path/to/module.js", "globalThis.my_value = 2;");
    /// let module = runtime.load_module(&module)?;
    /// let value: usize = runtime.get_value(Some(&module), "my_value")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_value<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.block_on(|runtime| async move { runtime.get_value_async(module_context, name).await })
    }

    /// Get a value from a runtime instance
    /// Returns a future that resolves when:
    /// - The event loop is resolved, and
    /// - If the value is a promise, the promise is resolved
    ///
    /// See [`crate::Runtime::get_value`] for an example
    ///
    /// # Arguments
    /// * `module_context` - Optional handle to a module to search - if None, or if the search fails, the global context is used
    /// * `name` - A string representing the name of the value to find
    ///
    /// # Returns
    /// A `Result` containing the deserialized result or an error (`Error`) if the value cannot be found,
    /// Or if the result cannot be deserialized into the requested type
    ///
    /// # Errors
    /// Can fail if the value cannot be found, or if the result cannot be deserialized.
    pub async fn get_value_async<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let result = self.inner.get_value_ref(module_context, name)?;
        let result = self.inner.resolve_with_event_loop(result).await?;
        self.inner.decode_value(result)
    }

    /// Get a value from a runtime instance
    /// Will not attempt to resolve promises, or run the event loop
    /// Promises can be returned by specifying the return type as [`crate::js_value::Promise`]
    /// The event loop should be run using [`crate::Runtime::await_event_loop`]
    ///
    /// # Arguments
    /// * `module_context` - Optional handle to a module to search - if None, or if the search fails, the global context is used
    /// * `name` - A string representing the name of the value to find
    ///
    /// # Returns
    /// A `Result` containing the deserialized result or an error (`Error`) if the value cannot be found,
    /// Or if the result cannot be deserialized into the requested type
    ///
    /// # Errors
    /// Can fail if the value cannot be found, or if the result cannot be deserialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustyscript::{ Runtime, Module, Error };
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("/path/to/module.js", "globalThis.my_value = 2;");
    /// let module = runtime.load_module(&module)?;
    /// let value: usize = runtime.get_value_immediate(Some(&module), "my_value")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_value_immediate<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let result = self.inner.get_value_ref(module_context, name)?;
        self.inner.decode_value(result)
    }

    /// Executes the given module, and returns a handle allowing you to extract values
    /// And call functions
    ///
    /// Blocks until the module has been executed AND the event loop has fully resolved
    /// See [`crate::Runtime::load_module_async`] for a non-blocking variant, or use with async
    /// background tasks
    ///
    /// # Arguments
    /// * `module` - A `Module` object containing the module's filename and contents.
    ///
    /// # Returns
    /// A `Result` containing a handle for the loaded module
    /// or an error (`Error`) if there are issues with loading or executing the module
    ///
    /// # Errors
    /// Can fail if the module cannot be loaded, or execution fails
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create a module with filename and contents
    /// use rustyscript::{Runtime, Module, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("test.js", "export default () => 'test'");
    /// runtime.load_module(&module);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_module(&mut self, module: &Module) -> Result<ModuleHandle, Error> {
        self.block_on(|runtime| async move {
            let handle = runtime.load_module_async(module).await;
            runtime
                .await_event_loop(PollEventLoopOptions::default(), None)
                .await?;
            handle
        })
    }

    /// Executes the given module, and returns a handle allowing you to extract values
    /// And call functions
    ///
    /// Returns a future that resolves to the handle for the loaded module
    /// Makes no attempt to fully resolve the event loop - call [`crate::Runtime::await_event_loop`]
    /// to resolve background tasks and async listeners
    ///
    /// # Arguments
    /// * `module` - A `Module` object containing the module's filename and contents.
    ///
    /// # Returns
    /// A `Result` containing a handle for the loaded module
    /// or an error (`Error`) if there are issues with loading or executing the module
    ///
    /// # Errors
    /// Can fail if the module cannot be loaded, or execution fails
    ///
    /// See [`crate::Runtime::load_module`] for an example
    pub async fn load_module_async(&mut self, module: &Module) -> Result<ModuleHandle, Error> {
        self.inner.load_modules(None, vec![module]).await
    }

    /// Executes the given module, and returns a handle allowing you to extract values
    /// And call functions.
    ///
    /// Blocks until all modules have been executed AND the event loop has fully resolved
    /// See [`crate::Runtime::load_module_async`] for a non-blocking variant, or use with async
    /// background tasks
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
    /// or an error (`Error`) if there are issues with loading or executing the module
    ///
    /// # Errors
    /// Can fail if the module cannot be loaded, or execution fails
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create a module with filename and contents
    /// use rustyscript::{Runtime, Module, Error};
    ///
    /// # fn main() -> Result<(), Error> {
    /// let mut runtime = Runtime::new(Default::default())?;
    /// let module = Module::new("test.js", "export default () => 'test'");
    /// runtime.load_modules(&module, vec![]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_modules(
        &mut self,
        module: &Module,
        side_modules: Vec<&Module>,
    ) -> Result<ModuleHandle, Error> {
        self.block_on(move |runtime| async move {
            let handle = runtime.load_modules_async(module, side_modules).await;
            runtime
                .await_event_loop(PollEventLoopOptions::default(), None)
                .await?;
            handle
        })
    }

    /// Executes the given module, and returns a handle allowing you to extract values
    /// And call functions.
    ///
    /// Returns a future that resolves to the handle for the loaded module
    /// Makes no attempt to resolve the event loop - call [`crate::Runtime::await_event_loop`] to
    /// resolve background tasks and async listeners
    ///
    /// This will load 'module' as the main module, and the others as side-modules.
    /// Only one main module can be loaded per runtime
    ///
    /// See [`crate::Runtime::load_modules`] for an example
    ///
    /// # Arguments
    /// * `module` - A `Module` object containing the module's filename and contents.
    /// * `side_modules` - A set of additional modules to be loaded into memory for use
    ///
    /// # Returns
    /// A `Result` containing a handle for the loaded main module, or the last side-module
    /// or an error (`Error`) if there are issues with loading or executing the modules
    ///
    /// # Errors
    /// Can fail if the modules cannot be loaded, or execution fails
    pub async fn load_modules_async(
        &mut self,
        module: &Module,
        side_modules: Vec<&Module>,
    ) -> Result<ModuleHandle, Error> {
        self.inner.load_modules(Some(module), side_modules).await
    }

    /// Executes the given module, on the runtime, making it available to be
    /// imported by other modules in this runtime, and those that will use the
    /// snapshot
    ///
    /// This is a blocking operation, and will run the event loop to completion
    /// For a non-blocking variant, see [`SnapshotBuilder::load_module_async`]
    ///
    /// # Arguments
    /// * `module` - A `Module` object containing the module's filename and contents.
    ///
    /// # Errors
    /// Can fail if the module cannot be loaded, or execution fails
    pub fn with_module(mut self, module: &Module) -> Result<Self, Error> {
        self.load_module(module)?;
        Ok(self)
    }

    /// Executes a piece of non-ECMAScript-module JavaScript code on the runtime
    /// This code can be used to set up the runtime state before creating the snapshot
    ///
    /// This is a blocking operation, and will run the event loop to completion
    ///
    /// # Arguments
    /// * `expr` - A string representing the JavaScript expression to evaluate
    ///
    /// # Errors
    /// Can fail if the expression cannot be evaluated, or if the result cannot be deserialized
    pub fn with_expression(mut self, expr: &str) -> Result<Self, Error> {
        self.eval::<()>(expr)?;
        Ok(self)
    }

    /// Consumes the runtime and returns a snapshot of the runtime state
    /// This is only available when the `snapshot_builder` feature is enabled
    /// and will return a `Box<[u8]>` representing the snapshot
    ///
    /// To use the snapshot, provide it, as a static slice, in [`RuntimeOptions::startup_snapshot`]
    /// Therefore, in order to use this snapshot, make sure you write it to a file and load it with
    /// `include_bytes!`
    ///
    /// WARNING: In order to use the snapshot, make sure the runtime using it is
    /// provided the same extensions and options as the original runtime. Any extensions
    /// you provided must be loaded with `init_ops` instead of `init_ops_and_esm`.
    #[must_use]
    pub fn finish(self) -> Box<[u8]> {
        let deno_rt: JsRuntimeForSnapshot = self.inner.into_inner();
        deno_rt.snapshot()
    }
}

impl AsyncBridgeExt for SnapshotBuilder {
    fn bridge(&self) -> &AsyncBridge {
        &self.tokio
    }
}
