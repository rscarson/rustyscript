use crate::{
    ext,
    module_loader::{LoaderOptions, RustyLoader},
    traits::{ToDefinedValue, ToModuleSpecifier, ToV8String},
    transpiler::transpile,
    utilities, Error, ExtensionOptions, Module, ModuleHandle,
};
use deno_core::{
    futures::FutureExt, serde_json, serde_v8::from_v8, v8, FeatureChecker, JsRuntime,
    JsRuntimeForSnapshot, PollEventLoopOptions,
};
use serde::de::DeserializeOwned;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    pin::Pin,
    rc::Rc,
    task::Poll,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

/// Wrapper trait to make the `InnerRuntime` generic over the runtime types
pub trait RuntimeTrait {
    fn try_new(options: deno_core::RuntimeOptions) -> Result<Self, Error>
    where
        Self: Sized;
    fn rt_mut(&mut self) -> &mut JsRuntime;
}
impl RuntimeTrait for JsRuntime {
    fn try_new(options: deno_core::RuntimeOptions) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let rt = Self::try_new(options)?;
        Ok(rt)
    }
    fn rt_mut(&mut self) -> &mut JsRuntime {
        self
    }
}
impl RuntimeTrait for JsRuntimeForSnapshot {
    fn try_new(options: deno_core::RuntimeOptions) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let rt = Self::try_new(options)?;
        Ok(rt)
    }
    fn rt_mut(&mut self) -> &mut JsRuntime {
        self
    }
}

/// Represents a function that can be registered with the runtime
pub trait RsFunction:
    Fn(&[serde_json::Value]) -> Result<serde_json::Value, Error> + 'static
{
}
impl<F> RsFunction for F where
    F: Fn(&[serde_json::Value]) -> Result<serde_json::Value, Error> + 'static
{
}

/// Represents an async function that can be registered with the runtime
pub trait RsAsyncFunction:
    Fn(
        Vec<serde_json::Value>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, Error>>>>
    + 'static
{
}
impl<F> RsAsyncFunction for F where
    F: Fn(
            Vec<serde_json::Value>,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, Error>>>>
        + 'static
{
}

/// Decodes a set of arguments into a vector of v8 values
/// This is used to pass arguments to a javascript function
/// And is faster and more flexible than using `json_args!`
fn decode_args<'a>(
    args: &impl serde::ser::Serialize,
    scope: &mut v8::HandleScope<'a>,
) -> Result<Vec<v8::Local<'a, v8::Value>>, Error> {
    let args = deno_core::serde_v8::to_v8(scope, args)?;
    match v8::Local::<v8::Array>::try_from(args) {
        Ok(args) => {
            let len = args.length();
            let mut result = Vec::with_capacity(len as usize);
            for i in 0..len {
                let index = v8::Integer::new(
                    scope,
                    i.try_into().map_err(|_| {
                        Error::Runtime(format!(
                            "Could not decode {len} arguments - use `big_json_args`"
                        ))
                    })?,
                );
                let arg = args
                    .get(scope, index.into())
                    .ok_or_else(|| Error::Runtime(format!("Invalid argument at index {i}")))?;
                result.push(arg);
            }
            Ok(result)
        }
        Err(_) if args.is_undefined() || args.is_null() => Ok(vec![]),
        Err(_) => Ok(vec![args]),
    }
}

/// Represents the set of options accepted by the runtime constructor
pub struct RuntimeOptions {
    /// A set of `deno_core` extensions to add to the runtime
    pub extensions: Vec<deno_core::Extension>,

    /// Additional options for the built-in extensions
    pub extension_options: ext::ExtensionOptions,

    /// Function to use as entrypoint if the module does not provide one
    pub default_entrypoint: Option<String>,

    /// Amount of time to run for before killing the thread
    pub timeout: Duration,

    /// Optional maximum heap size for the runtime
    pub max_heap_size: Option<usize>,

    /// Optional cache provider for the module loader
    #[allow(deprecated)]
    pub module_cache: Option<Box<dyn crate::module_loader::ModuleCacheProvider>>,

    /// Optional import provider for the module loader
    pub import_provider: Option<Box<dyn crate::module_loader::ImportProvider>>,

    /// Optional snapshot to load into the runtime
    ///
    /// This will reduce load times, but requires the same extensions to be loaded as when the snapshot was created  
    /// If provided, user-supplied extensions must be instantiated with `init_ops` instead of `init_ops_and_esm`
    ///
    /// WARNING: Snapshots MUST be used on the same system they were created on
    pub startup_snapshot: Option<&'static [u8]>,

    /// Optional configuration parameters for building the underlying v8 isolate
    ///
    /// This can be used to alter the behavior of the runtime.
    ///
    /// See the `rusty_v8` documentation for more information
    pub isolate_params: Option<v8::CreateParams>,

    /// Optional shared array buffer store to use for the runtime.
    ///
    /// Allows data-sharing between runtimes across threads
    pub shared_array_buffer_store: Option<deno_core::SharedArrayBufferStore>,

    /// A whitelist of custom schema prefixes that are allowed to be loaded from javascript
    ///
    /// By default only `http`/`https` (`url_import` crate feature), and `file` (`fs_import` crate feature) are allowed
    pub schema_whlist: HashSet<String>,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            extensions: Vec::default(),
            default_entrypoint: None,
            timeout: Duration::MAX,
            max_heap_size: None,
            module_cache: None,
            import_provider: None,
            startup_snapshot: None,
            isolate_params: None,
            shared_array_buffer_store: None,
            schema_whlist: HashSet::default(),

            extension_options: ExtensionOptions::default(),
        }
    }
}

/// Deno `JsRuntime` wrapper providing helper functions needed
/// by the public-facing Runtime API
///
/// This struct is not intended to be used directly by the end user
/// It provides a set of async functions that can be used to interact with the
/// underlying deno runtime instance
pub struct InnerRuntime<RT: RuntimeTrait> {
    pub module_loader: Rc<RustyLoader>,
    pub deno_runtime: RT,

    pub cwd: PathBuf,
    pub default_entrypoint: Option<String>,
}
impl<RT: RuntimeTrait> InnerRuntime<RT> {
    pub fn new(
        options: RuntimeOptions,
        heap_exhausted_token: CancellationToken,
    ) -> Result<Self, Error> {
        let cwd = std::env::current_dir()?;
        let module_loader = Rc::new(RustyLoader::new(LoaderOptions {
            cache_provider: options.module_cache,
            import_provider: options.import_provider,
            schema_whlist: options.schema_whlist,
            cwd: cwd.clone(),

            #[cfg(feature = "node_experimental")]
            node_resolver: options.extension_options.node_resolver.clone(),

            ..Default::default()
        }));

        // Init otel
        #[cfg(feature = "web")]
        {
            let otel_conf = options.extension_options.web.telemetry_config.clone();
            deno_telemetry::init(otel_conf)?;
        }

        // If a snapshot is provided, do not reload ESM for extensions
        let is_snapshot = options.startup_snapshot.is_some();
        let extensions = ext::all_extensions(
            options.extensions,
            options.extension_options,
            options.shared_array_buffer_store.clone(),
            is_snapshot,
        );

        // If a heap size is provided, set the isolate params (preserving any user-provided params otherwise)
        let isolate_params = match options.isolate_params {
            Some(params) => {
                if let Some(max_heap_size) = options.max_heap_size {
                    Some(params.heap_limits(0, max_heap_size))
                } else {
                    Some(params)
                }
            }
            None => {
                if let Some(max_heap_size) = options.max_heap_size {
                    let params = v8::Isolate::create_params().heap_limits(0, max_heap_size);
                    Some(params)
                } else {
                    None
                }
            }
        };

        let mut feature_checker = FeatureChecker::default();
        feature_checker.set_exit_cb(Box::new(|_, _| {}));

        let mut deno_runtime = RT::try_new(deno_core::RuntimeOptions {
            module_loader: Some(module_loader.clone()),

            feature_checker: Some(feature_checker.into()),

            extension_transpiler: Some(module_loader.as_extension_transpiler()),
            create_params: isolate_params,
            shared_array_buffer_store: options.shared_array_buffer_store.clone(),

            startup_snapshot: options.startup_snapshot,
            extensions,

            ..Default::default()
        })?;

        // Add a callback to terminate the runtime if the max_heap_size limit is approached
        if options.max_heap_size.is_some() {
            let isolate_handle = deno_runtime.rt_mut().v8_isolate().thread_safe_handle();

            deno_runtime
                .rt_mut()
                .add_near_heap_limit_callback(move |current_value, _| {
                    isolate_handle.terminate_execution();

                    // Signal the outer runtime to cancel block_on future (avoid hanging) and return friendly error
                    heap_exhausted_token.cancel();

                    // Spike the heap limit while terminating to avoid segfaulting
                    // Callback may fire multiple times if memory usage increases quicker then termination finalizes
                    5 * current_value
                });
        }

        let default_entrypoint = options.default_entrypoint;
        Ok(Self {
            module_loader,
            deno_runtime,
            cwd,
            default_entrypoint,
        })
    }

    /// Destroy the `RustyScript` runtime, returning the deno RT instance
    #[allow(dead_code)]
    pub fn into_inner(self) -> RT {
        self.deno_runtime
    }

    /// Access the underlying deno runtime instance directly
    pub fn deno_runtime(&mut self) -> &mut JsRuntime {
        self.deno_runtime.rt_mut()
    }

    /// Set the current working directory for the runtime
    /// This is used to resolve relative paths in the module loader
    pub fn set_current_dir(&mut self, path: impl AsRef<Path>) -> Result<&Path, Error> {
        let path = path.as_ref();
        let path = utilities::resolve_path(path, Some(&self.cwd))?
            .to_file_path()
            .map_err(|()| Error::Runtime("Invalid path".to_string()))?;

        self.cwd = path;
        self.module_loader.set_current_dir(self.cwd.clone());
        Ok(&self.cwd)
    }

    pub fn current_dir(&self) -> &Path {
        &self.cwd
    }

    /// Remove and return a value from the state
    pub fn take<T>(&mut self) -> Option<T>
    where
        T: 'static,
    {
        let state = self.deno_runtime().op_state();
        if let Ok(mut state) = state.try_borrow_mut() {
            if state.has::<T>() {
                return Some(state.take());
            }
        }

        None
    }

    /// Add a value to the state
    /// Only one value of each type is stored
    pub fn put<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: 'static,
    {
        let state = self.deno_runtime().op_state();
        let mut state = state.try_borrow_mut()?;
        state.put(value);

        Ok(())
    }

    /// Register an async rust function
    /// The function must return a Future that resolves to a `serde_json::Value`
    /// and accept a vec of `serde_json::Value` as arguments
    pub fn register_async_function<F>(&mut self, name: &str, callback: F) -> Result<(), Error>
    where
        F: RsAsyncFunction,
    {
        let state = self.deno_runtime().op_state();
        let mut state = state.try_borrow_mut()?;

        if !state.has::<HashMap<String, Box<dyn RsAsyncFunction>>>() {
            state.put(HashMap::<String, Box<dyn RsAsyncFunction>>::new());
        }

        // Insert the callback into the state
        state
            .borrow_mut::<HashMap<String, Box<dyn RsAsyncFunction>>>()
            .insert(name.to_string(), Box::new(callback));

        Ok(())
    }

    /// Register a rust function
    /// The function must return a `serde_json::Value`
    /// and accept a slice of `serde_json::Value` as arguments
    pub fn register_function<F>(&mut self, name: &str, callback: F) -> Result<(), Error>
    where
        F: RsFunction,
    {
        let state = self.deno_runtime().op_state();
        let mut state = state.try_borrow_mut()?;

        if !state.has::<HashMap<String, Box<dyn RsFunction>>>() {
            state.put(HashMap::<String, Box<dyn RsFunction>>::new());
        }

        // Insert the callback into the state
        state
            .borrow_mut::<HashMap<String, Box<dyn RsFunction>>>()
            .insert(name.to_string(), Box::new(callback));

        Ok(())
    }

    /// Runs the JS event loop to completion
    pub async fn await_event_loop(
        &mut self,
        options: PollEventLoopOptions,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        if let Some(timeout) = timeout {
            Ok(tokio::select! {
                r = self.deno_runtime().run_event_loop(options) => r,
                () = tokio::time::sleep(timeout) => Ok(()),
            }?)
        } else {
            Ok(self.deno_runtime().run_event_loop(options).await?)
        }
    }

    /// Advances the JS event loop by one tick
    /// Return true if the event loop is pending
    pub async fn advance_event_loop(
        &mut self,
        options: PollEventLoopOptions,
    ) -> Result<bool, Error> {
        let result = std::future::poll_fn(|cx| {
            Poll::Ready(match self.deno_runtime().poll_event_loop(cx, options) {
                Poll::Ready(t) => t.map(|()| false),
                Poll::Pending => Ok(true),
            })
        })
        .await?;

        Ok(result)
    }

    /// Evaluate a piece of non-ECMAScript-module JavaScript code
    /// The expression is evaluated in the global context, so changes persist
    ///
    /// Async because some expressions may require a tokio runtime
    ///
    /// # Arguments
    /// * `expr` - A string representing the JavaScript expression to evaluate
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the expression (`T`)
    /// or an error (`Error`) if the expression cannot be evaluated or if the
    /// result cannot be deserialized.
    #[allow(clippy::unused_async, reason = "Prevent panic on sleep calls")]
    pub async fn eval(&mut self, expr: impl ToString) -> Result<v8::Global<v8::Value>, Error> {
        let result = self.deno_runtime().execute_script("", expr.to_string())?;
        Ok(result)
    }

    /// Attempt to get a value out of the global context (globalThis.name)
    ///
    /// # Arguments
    /// * `name` - Name of the object to extract
    ///
    /// # Returns
    /// A `Result` containing the non-null value extracted or an error (`Error`)
    pub fn get_global_value(&mut self, name: &str) -> Result<v8::Global<v8::Value>, Error> {
        let context = self.deno_runtime().main_context();
        let mut scope = self.deno_runtime().handle_scope();
        let global = context.open(&mut scope).global(&mut scope);

        let key = name.to_v8_string(&mut scope)?;
        let value = global.get(&mut scope, key.into());

        match value.if_defined() {
            Some(v) => Ok(v8::Global::<v8::Value>::new(&mut scope, v)),
            _ => Err(Error::ValueNotFound(name.to_string())),
        }
    }

    /// Attempt to get a value out of a module context
    ///     ///
    /// # Arguments
    /// * `module` - A handle to a loaded module
    /// * `name` - Name of the object to extract
    ///
    /// # Returns
    /// A `Result` containing the non-null value extracted or an error (`Error`)
    pub fn get_module_export_value(
        &mut self,
        module_context: &ModuleHandle,
        name: &str,
    ) -> Result<v8::Global<v8::Value>, Error> {
        let module_namespace = self
            .deno_runtime()
            .get_module_namespace(module_context.id())?;
        let mut scope = self.deno_runtime().handle_scope();
        let module_namespace = module_namespace.open(&mut scope);
        assert!(module_namespace.is_module_namespace_object());

        let key = name.to_v8_string(&mut scope)?;
        let value = module_namespace.get(&mut scope, key.into());

        match value.if_defined() {
            Some(v) => Ok(v8::Global::<v8::Value>::new(&mut scope, v)),
            _ => Err(Error::ValueNotFound(name.to_string())),
        }
    }

    pub async fn resolve_with_event_loop(
        &mut self,
        value: v8::Global<v8::Value>,
    ) -> Result<v8::Global<v8::Value>, Error> {
        let future = self.deno_runtime().resolve(value);
        let result = self
            .deno_runtime()
            .with_event_loop_future(future, PollEventLoopOptions::default())
            .await?;
        Ok(result)
    }

    pub fn decode_value<T>(&mut self, value: v8::Global<v8::Value>) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let mut scope = self.deno_runtime().handle_scope();
        let result = v8::Local::<v8::Value>::new(&mut scope, value);
        Ok(from_v8(&mut scope, result)?)
    }

    pub fn get_value_ref(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
    ) -> Result<v8::Global<v8::Value>, Error> {
        // Try to get the value from the module context first
        let result = module_context
            .and_then(|module_context| self.get_module_export_value(module_context, name).ok());

        // If it's not found, try the global context
        match result {
            Some(result) => Ok(result),
            None => self
                .get_global_value(name)
                .map_err(|_| Error::ValueNotFound(name.to_string())),
        }
    }

    /// Retrieves a javascript function by its name from the Deno runtime's global context.
    ///
    /// # Arguments
    /// * `module_context` - A module handle to use for context, to find exports
    /// * `name` - A string representing the name of the javascript function to retrieve.
    ///
    /// # Returns
    /// A `Result` containing a `v8::Global<v8::Function>` if
    /// the function is found, or an error (`Error`) if the function cannot be found or
    /// if it is not a valid javascript function.
    pub fn get_function_by_name(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
    ) -> Result<v8::Global<v8::Function>, Error> {
        // Get the value
        let value = self.get_value_ref(module_context, name)?;

        // Convert it into a function
        let mut scope = self.deno_runtime().handle_scope();
        let local_value = v8::Local::<v8::Value>::new(&mut scope, value);
        let f: v8::Local<v8::Function> = local_value
            .try_into()
            .or::<Error>(Err(Error::ValueNotCallable(name.to_string())))?;

        // Return it as a global
        Ok(v8::Global::<v8::Function>::new(&mut scope, f))
    }

    pub fn call_function_by_ref(
        &mut self,
        module_context: Option<&ModuleHandle>,
        function: &v8::Global<v8::Function>,
        args: &impl serde::ser::Serialize,
    ) -> Result<v8::Global<v8::Value>, Error> {
        // Namespace, if provided
        let module_namespace = if let Some(module_context) = module_context {
            Some(
                self.deno_runtime()
                    .get_module_namespace(module_context.id())?,
            )
        } else {
            None
        };

        let mut scope = self.deno_runtime().handle_scope();
        let mut scope = v8::TryCatch::new(&mut scope);

        // Get the namespace
        // Module-level if supplied, none otherwise
        let namespace: v8::Local<v8::Value> = if let Some(namespace) = module_namespace {
            v8::Local::<v8::Object>::new(&mut scope, namespace).into()
        } else {
            // Create a new object to use as the namespace if none is provided
            //let obj: v8::Local<v8::Value> = v8::Object::new(&mut scope).into();
            let obj: v8::Local<v8::Value> = v8::undefined(&mut scope).into();
            obj
        };

        let function_instance = function.open(&mut scope);

        // Prep arguments
        let args = decode_args(args, &mut scope)?;

        // Call the function
        let result = function_instance.call(&mut scope, namespace, &args);
        match result {
            Some(value) => {
                let value = v8::Global::new(&mut scope, value);
                Ok(value)
            }
            None if scope.has_caught() => {
                let e = scope
                    .message()
                    .ok_or_else(|| Error::Runtime("Unknown error".to_string()))?;

                let filename = e.get_script_resource_name(&mut scope);
                let linenumber = e.get_line_number(&mut scope).unwrap_or_default();
                let filename = if let Some(v) = filename {
                    let filename = v.to_rust_string_lossy(&mut scope);
                    format!("{filename}:{linenumber}: ")
                } else if let Some(module_context) = module_context {
                    let filename = module_context.module().filename().to_string_lossy();
                    format!("{filename}:{linenumber}: ")
                } else {
                    String::new()
                };

                let msg = e.get(&mut scope).to_rust_string_lossy(&mut scope);

                let s = format!("{filename}{msg}");
                Err(Error::Runtime(s))
            }
            None => Err(Error::Runtime(
                "Unknown error during function execution".to_string(),
            )),
        }
    }

    /// A utility function that run provided future concurrently with the event loop.
    ///
    /// If the event loop resolves while polling the future, it will continue to be polled,
    /// Unless it returned an error
    ///
    /// Useful for interacting with local inspector session.
    pub async fn with_event_loop_future<'fut, T, E>(
        &mut self,
        mut fut: impl std::future::Future<Output = Result<T, E>> + Unpin + 'fut,
        poll_options: PollEventLoopOptions,
    ) -> Result<T, Error>
    where
        deno_core::error::AnyError: From<E>,
        Error: std::convert::From<E>,
    {
        // Manually implement tokio::select
        std::future::poll_fn(|cx| {
            if let Poll::Ready(t) = fut.poll_unpin(cx) {
                return if let Poll::Ready(Err(e)) =
                    self.deno_runtime().poll_event_loop(cx, poll_options)
                {
                    // Run one more tick to check for errors
                    Poll::Ready(Err(e.into()))
                } else {
                    // No errors - continue
                    Poll::Ready(t.map_err(Into::into))
                };
            }

            if let Poll::Ready(Err(e)) = self.deno_runtime().poll_event_loop(cx, poll_options) {
                // Event loop failed
                return Poll::Ready(Err(e.into()));
            }

            if self
                .deno_runtime()
                .poll_event_loop(cx, poll_options)
                .is_ready()
            {
                // Event loop resolved - continue
                println!("Event loop resolved");
            }

            Poll::Pending
        })
        .await
    }

    /// Get the entrypoint function for a module
    pub fn get_module_entrypoint(
        &mut self,
        module_context: &mut ModuleHandle,
    ) -> Result<Option<v8::Global<v8::Function>>, Error> {
        let default = self.default_entrypoint.clone();

        // Try to get an entrypoint from a call to `rustyscript.register_entrypoint` first
        let state = self.deno_runtime().op_state();
        let mut deep_state = state.try_borrow_mut()?;
        let entrypoint = deep_state.try_take::<v8::Global<v8::Function>>();
        if let Some(entrypoint) = entrypoint {
            return Ok(Some(entrypoint));
        }

        // Try to get an entrypoint from the default export next
        if let Ok(default_export) = self.get_module_export_value(module_context, "default") {
            let mut scope = self.deno_runtime().handle_scope();
            let default_export = v8::Local::new(&mut scope, default_export);
            if default_export.is_function() {
                if let Ok(f) = v8::Local::<v8::Function>::try_from(default_export) {
                    return Ok(Some(v8::Global::new(&mut scope, f)));
                }
            }
        }

        // Try to get an entrypoint from the default entrypoint
        if let Some(default) = default.as_deref() {
            if let Ok(f) = self.get_function_by_name(Some(module_context), default) {
                return Ok(Some(f));
            }
        }

        Ok(None)
    }

    /// Load one or more modules
    /// Returns a future that resolves to a handle to the main module, or the last
    /// side-module
    ///
    /// Will return a handle to the main module, or the last
    /// side-module
    pub async fn load_modules(
        &mut self,
        main_module: Option<&Module>,
        side_modules: Vec<&Module>,
    ) -> Result<ModuleHandle, Error> {
        if main_module.is_none() && side_modules.is_empty() {
            return Err(Error::Runtime(
                "Internal error: attempt to load no modules".to_string(),
            ));
        }

        let mut module_handle_stub = ModuleHandle::default();

        // Get additional modules first
        for side_module in side_modules {
            let module_specifier = side_module.filename().to_module_specifier(&self.cwd)?;
            let (code, sourcemap) = transpile(&module_specifier, side_module.contents())?;

            // Now CJS translation, for node
            #[cfg(feature = "node_experimental")]
            let code = self
                .module_loader
                .translate_cjs(&module_specifier, &code)
                .await?;

            let fast_code = deno_core::FastString::from(code.clone());

            let s_modid = self
                .deno_runtime()
                .load_side_es_module_from_code(&module_specifier, fast_code)
                .await?;

            // Update source map cache
            self.module_loader.insert_source_map(
                module_specifier.as_str(),
                code,
                sourcemap.map(|s| s.to_vec()),
            );

            let mod_load = self.deno_runtime().mod_evaluate(s_modid);
            self.with_event_loop_future(mod_load, PollEventLoopOptions::default())
                .await?;
            module_handle_stub = ModuleHandle::new(side_module, s_modid, None);
        }

        // Load main module
        if let Some(module) = main_module {
            let module_specifier = module.filename().to_module_specifier(&self.cwd)?;
            let (code, sourcemap) = transpile(&module_specifier, module.contents())?;

            // Now CJS translation, for node
            #[cfg(feature = "node_experimental")]
            let code = self
                .module_loader
                .translate_cjs(&module_specifier, &code)
                .await?;

            let fast_code = deno_core::FastString::from(code.clone());

            let module_id = self
                .deno_runtime()
                .load_main_es_module_from_code(&module_specifier, fast_code)
                .await?;

            // Update source map cache
            self.module_loader.insert_source_map(
                module_specifier.as_str(),
                code,
                sourcemap.map(|s| s.to_vec()),
            );

            // Finish execution
            let mod_load = self.deno_runtime().mod_evaluate(module_id);
            self.with_event_loop_future(mod_load, PollEventLoopOptions::default())
                .await?;
            module_handle_stub = ModuleHandle::new(module, module_id, None);
        }

        // Try to get the default entrypoint
        let entrypoint = self.get_module_entrypoint(&mut module_handle_stub)?;

        Ok(ModuleHandle::new(
            module_handle_stub.module(),
            module_handle_stub.id(),
            entrypoint,
        ))
    }
}

#[cfg(test)]
mod test_inner_runtime {
    use serde::Deserialize;

    use crate::{async_callback, big_json_args, js_value::Function, json_args, sync_callback};

    #[cfg(any(feature = "web", feature = "web_stub"))]
    use crate::js_value::Promise;

    use super::*;

    /// Used for blocking functions
    fn run_async_task<T, F, U>(f: F) -> T
    where
        U: std::future::Future<Output = Result<T, Error>>,
        F: FnOnce() -> U,
    {
        let timeout = Duration::from_secs(2);
        let tokio = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .thread_keep_alive(timeout)
            .build()
            .unwrap();
        tokio
            .block_on(async move {
                tokio::time::timeout(timeout, f())
                    .await
                    .expect("Test failed")
            })
            .expect("Timed out")
    }

    macro_rules! assert_v8 {
        ($l:expr, $r:expr, $t:ty, $rt:expr) => {
            assert_eq!($rt.decode_value::<$t>($l).expect("Wrong type"), $r,)
        };
    }

    #[test]
    fn test_decode_args() {
        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");
        let mut scope = runtime.deno_runtime.handle_scope();

        // empty
        let args = decode_args(&json_args!(), &mut scope).expect("Could not decode args");
        assert_eq!(args.len(), 0);

        // single
        let args = decode_args(&json_args!(2), &mut scope).expect("Could not decode args");
        assert_eq!(args.len(), 1);

        // single raw
        let args = decode_args(&2, &mut scope).expect("Could not decode args");
        assert_eq!(args.len(), 1);

        // multiple heterogeneous
        let args = decode_args(&json_args!(2, "test"), &mut scope).expect("Could not decode args");
        assert_eq!(args.len(), 2);

        // multiple homogeneous
        let args = decode_args(&json_args!(2, 3), &mut scope).expect("Could not decode args");
        assert_eq!(args.len(), 2);

        // 16 args
        let args = decode_args(
            &(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15),
            &mut scope,
        )
        .expect("Could not decode args");
        assert_eq!(args.len(), 16);

        // 32 args
        let args = decode_args(
            &big_json_args!(
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
                10, 11, 12, 13, 14, 15
            ),
            &mut scope,
        )
        .expect("Could not decode args");
        assert_eq!(args.len(), 32);
    }

    #[test]
    fn test_put_take() {
        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        runtime.put(2usize).expect("Could not put value");
        let v = runtime.take::<usize>().expect("Could not take value");
        assert_eq!(v, 2);
    }

    #[test]
    fn test_register_async_function() {
        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");
        runtime
            .register_async_function(
                "test",
                async_callback!(|a: i64, b: i64| async move { Ok::<i64, Error>(a + b) }),
            )
            .expect("Could not register function");

        let module = Module::new(
            "test.js",
            "
            globalThis.v = await rustyscript.async_functions.test(2, 3);
            ",
        );

        let rt = &mut runtime;
        let module = run_async_task(|| async move { rt.load_modules(Some(&module), vec![]).await });

        let result = runtime
            .get_value_ref(Some(&module), "v")
            .expect("Could not find global");
        assert_v8!(result, 5, usize, runtime);
    }

    #[test]
    fn test_register_function() {
        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");
        runtime
            .register_function(
                "test",
                sync_callback!(|a: i64, b: i64| { Ok::<i64, Error>(a + b) }),
            )
            .expect("Could not register function");

        run_async_task(|| async move {
            let v = runtime
                .eval("rustyscript.functions.test(2, 3)")
                .await
                .expect("failed to eval");
            assert_v8!(v, 5, usize, runtime);
            Ok(())
        });
    }

    #[cfg(any(feature = "web", feature = "web_stub"))]
    #[test]
    fn test_eval() {
        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        run_async_task(|| async move {
            let v = runtime.eval("2 + 2").await.expect("failed to eval");
            assert_v8!(v, 4, usize, runtime);
            let result = runtime
                .eval(
                    "
                let sleep = (ms) => new Promise((r) => setTimeout(r, ms));
                sleep(500).then(() => 2);
            ",
                )
                .await
                .expect("failed to eval");

            let result: Promise<u32> = runtime
                .decode_value(result)
                .expect("Could not decode promise");

            let result: u32 = result.resolve(runtime.deno_runtime()).await?;
            assert_eq!(result, 2);
            Ok(())
        });
    }

    #[cfg(feature = "web_stub")]
    #[test]
    fn test_base64() {
        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        run_async_task(|| async move {
            let result = runtime.eval("btoa('foo')").await.expect("failed to eval");
            assert_v8!(result, "Zm9v", String, runtime);

            let result = runtime
                .eval("atob(btoa('foo'))")
                .await
                .expect("failed to eval");
            assert_v8!(result, "foo", String, runtime);

            Ok(())
        });
    }

    #[test]
    fn test_get_value_ref() {
        let module = Module::new(
            "test.js",
            "
            globalThis.a = 2;
            export const b = 'test';
            export const fnc = null;
        ",
        );

        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        let rt = &mut runtime;
        let module = run_async_task(|| async move { rt.load_modules(Some(&module), vec![]).await });

        let v = runtime
            .get_value_ref(None, "a")
            .expect("Could not find global");
        assert_v8!(v, 2, usize, runtime);

        let v = runtime
            .get_value_ref(Some(&module), "a")
            .expect("Could not find global");
        assert_v8!(v, 2, usize, runtime);

        let v = runtime
            .get_value_ref(Some(&module), "b")
            .expect("Could not find export");
        assert_v8!(v, "test", String, runtime);

        runtime
            .get_value_ref(Some(&module), "c")
            .expect_err("Could not detect null");

        runtime
            .get_value_ref(Some(&module), "d")
            .expect_err("Could not detect undeclared");
    }

    #[test]
    fn test_get_function_by_name() {
        let module = Module::new(
            "test.js",
            "
            globalThis.fna = () => {};
            export function fnb() {}
            export const fnc = 2;
        ",
        );

        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        let rt = &mut runtime;
        let module = run_async_task(|| async move { rt.load_modules(Some(&module), vec![]).await });

        runtime
            .get_function_by_name(Some(&module), "fna")
            .expect("Did not find global");
        runtime
            .get_function_by_name(Some(&module), "fnb")
            .expect("Did not find export");
        runtime
            .get_function_by_name(Some(&module), "fnc")
            .expect_err("Did not detect non-function");
        runtime
            .get_function_by_name(Some(&module), "fnd")
            .expect_err("Did not detect undefined");
    }

    #[test]
    fn test_call_function_by_ref() {
        let module = Module::new(
            "test.js",
            "
            globalThis.fna = (i) => i;
            export function fnb() {
                return 'test';
            }
            export const fnc = 2;
            export const fne = () => {};

            export const will_err = () => {
                throw new Error('msg');
            }
        ",
        );

        run_async_task(|| async move {
            let mut runtime =
                InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                    .expect("Could not load runtime");
            let handle = runtime.load_modules(Some(&module), vec![]).await?;

            let f = runtime.get_function_by_name(None, "fna").unwrap();
            let result = runtime
                .call_function_by_ref(Some(&handle), &f, json_args!(2))
                .expect("Could not call global");
            assert_v8!(result, 2, usize, runtime);

            let f = runtime.get_function_by_name(Some(&handle), "fnb").unwrap();
            let result = runtime
                .call_function_by_ref(Some(&handle), &f, json_args!())
                .expect("Could not call export");
            assert_v8!(result, "test", String, runtime);

            let f = runtime.get_function_by_name(Some(&handle), "fne").unwrap();
            runtime
                .call_function_by_ref(Some(&handle), &f, json_args!())
                .expect("Did not allow undefined return");

            let f = runtime
                .get_function_by_name(Some(&handle), "will_err")
                .unwrap();
            runtime
                .call_function_by_ref(Some(&handle), &f, json_args!())
                .expect_err("Did not catch error");

            Ok(())
        });
    }

    #[test]
    fn test_ts_loader() {
        let module = Module::new(
            "test.ts",
            "
            export function test(left:number, right:number): number {
                return left + right;
            }
        ",
        );

        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        let rt = &mut runtime;
        let module = run_async_task(|| async move { rt.load_modules(Some(&module), vec![]).await });

        let f = runtime.get_function_by_name(Some(&module), "test").unwrap();
        let rt = &mut runtime;
        let result = run_async_task(|| async move {
            rt.call_function_by_ref(Some(&module), &f, json_args!(2, 3))
        });
        assert_v8!(result, 5, usize, runtime);
    }

    #[cfg(any(feature = "web", feature = "web_stub"))]
    #[test]
    fn test_toplevel_await() {
        let module = Module::new(
            "test.js",
            "
            const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
            await sleep(100);
            export function test() {
                return 2;
            }
        ",
        );

        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        let rt = &mut runtime;
        let module = run_async_task(|| async move {
            let h = rt.load_modules(Some(&module), vec![]).await;
            rt.await_event_loop(PollEventLoopOptions::default(), None)
                .await?;
            h
        });

        let f = runtime.get_function_by_name(Some(&module), "test").unwrap();
        let rt = &mut runtime;
        let result =
            run_async_task(
                || async move { rt.call_function_by_ref(Some(&module), &f, json_args!()) },
            );
        assert_v8!(result, 2, usize, runtime);
    }

    #[cfg(any(feature = "web", feature = "web_stub"))]
    #[test]
    fn test_promise() {
        let module = Module::new(
            "test.js",
            "
            export const test = () => {
                return new Promise((resolve) => {
                    setTimeout(() => {
                        resolve(2);
                    }, 50);
                });
            }
        ",
        );

        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        let rt = &mut runtime;
        run_async_task(|| async move {
            let module = rt.load_modules(Some(&module), vec![]).await?;

            let f = rt.get_function_by_name(Some(&module), "test").unwrap();
            let result = rt.call_function_by_ref(Some(&module), &f, json_args!())?;

            let result = rt.resolve_with_event_loop(result).await?;
            assert_v8!(result, 2, usize, rt);

            Ok(())
        });
    }

    #[cfg(any(feature = "web", feature = "web_stub"))]
    #[test]
    fn test_async_fn() {
        let module = Module::new(
            "test.js",
            "
            const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
            export async function test() {
                await sleep(100);
                return 2;
            }
        ",
        );

        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        let rt = &mut runtime;
        run_async_task(|| async move {
            let module = rt.load_modules(Some(&module), vec![]).await?;

            let f = rt.get_function_by_name(Some(&module), "test")?;
            let result = rt.call_function_by_ref(Some(&module), &f, json_args!())?;
            let result: Promise<usize> = rt.decode_value(result).expect("Could not deserialize");
            let result: usize = result.resolve(rt.deno_runtime()).await?;
            assert_eq!(2, result);

            Ok(())
        });
    }

    #[test]
    fn test_serialize_deep_fn() {
        let module = Module::new(
            "test.js",
            "
            let a = 2;
            export const test = {
                'name': 'test',
                'func': (x) => x + a
            }
        ",
        );

        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        let rt = &mut runtime;
        let module = run_async_task(|| async move { rt.load_modules(Some(&module), vec![]).await });

        #[derive(Deserialize)]
        #[allow(clippy::items_after_statements)]
        struct TestStruct {
            #[allow(dead_code)]
            name: String,
            func: Function,
        }

        let structure = runtime.get_value_ref(Some(&module), "test").unwrap();
        let structure: TestStruct = runtime
            .decode_value(structure)
            .expect("Could not deserialize");

        let function = structure
            .func
            .as_global(&mut runtime.deno_runtime().handle_scope());

        run_async_task(|| async move {
            let value = runtime
                .call_function_by_ref(Some(&module), &function, json_args!(2))
                .expect("could not call function");
            assert_v8!(value, 4, usize, runtime);

            let value = runtime
                .call_function_by_ref(Some(&module), &function, json_args!(3))
                .expect("could not call function twice");
            assert_v8!(value, 5, usize, runtime);

            Ok(())
        });
    }

    #[test]
    fn test_async_load_errors() {
        let module = Module::new(
            "test.js",
            "
            throw new Error('msg');
        ",
        );

        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        let rt = &mut runtime;
        let module_ = module.clone();
        let result =
            run_async_task(
                || async move { Ok(rt.load_modules(Some(&module_), vec![]).await.is_err()) },
            );
        assert!(result);

        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        let rt = &mut runtime;
        let result =
            run_async_task(
                || async move { Ok(rt.load_modules(None, vec![&module]).await.is_err()) },
            );
        assert!(result);
    }

    #[test]
    fn test_serialize_fn() {
        let module = Module::new(
            "test.js",
            "
            export const test = (x) => 2*x;
        ",
        );

        let mut runtime =
            InnerRuntime::<JsRuntime>::new(RuntimeOptions::default(), CancellationToken::new())
                .expect("Could not load runtime");

        let rt = &mut runtime;
        let module = run_async_task(|| async move { rt.load_modules(Some(&module), vec![]).await });

        let function = runtime
            .get_function_by_name(Some(&module), "test")
            .expect("Could not get function");

        run_async_task(|| async move {
            let value = runtime
                .call_function_by_ref(Some(&module), &function, json_args!(2))
                .expect("could not call function");
            assert_v8!(value, 4, usize, runtime);

            let value = runtime
                .call_function_by_ref(None, &function, json_args!(2))
                .expect("could not call function");
            assert_v8!(value, 4, usize, runtime);

            Ok(())
        });
    }
}
