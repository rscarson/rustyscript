use crate::{
    cache_provider::ModuleCacheProvider,
    ext,
    js_function::JsFunction,
    module_loader::RustyLoader,
    traits::{ToDefinedValue, ToModuleSpecifier, ToV8String},
    transpiler::{self, transpile_extension},
    Error, Module, ModuleHandle,
};
use deno_core::{serde_json, v8, JsRuntime, OpState, PollEventLoopOptions, RuntimeOptions};
use std::{collections::HashMap, pin::Pin, rc::Rc, time::Duration};

/// Callback type for rust callback functions
pub type RsFunction = fn(&FunctionArguments, &mut OpState) -> Result<serde_json::Value, Error>;

/// Callback type for async rust callback functions
pub type RsAsyncFunction = Box<
    dyn Fn(
        Vec<serde_json::Value>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, Error>>>>,
>;

/// Type required to pass arguments to JsFunctions
pub type FunctionArguments = [serde_json::Value];

/// Represents the set of options accepted by the runtime constructor
pub struct InnerRuntimeOptions {
    /// A set of deno_core extensions to add to the runtime
    pub extensions: Vec<deno_core::Extension>,

    /// Additional options for the built-in extensions
    pub extension_options: ext::ExtensionOptions,

    /// Function to use as entrypoint if the module does not provide one
    pub default_entrypoint: Option<String>,

    /// Amount of time to run for before killing the thread
    pub timeout: Duration,

    /// Optional cache provider for the module loader
    pub module_cache: Option<Box<dyn ModuleCacheProvider>>,

    /// Optional snapshot to load into the runtime
    /// This will reduce load times, but requires the same extensions to be loaded
    /// as when the snapshot was created
    /// If provided, user-supplied extensions must be instantiated with `init_ops` instead of `init_ops_and_esm`
    pub startup_snapshot: Option<&'static [u8]>,
}

impl Default for InnerRuntimeOptions {
    fn default() -> Self {
        Self {
            extensions: Default::default(),
            default_entrypoint: Default::default(),
            timeout: Duration::MAX,
            module_cache: None,
            startup_snapshot: None,

            extension_options: Default::default(),
        }
    }
}

/// Deno JsRuntime wrapper providing helper functions needed
/// by the public-facing Runtime API
pub struct InnerRuntime {
    pub deno_runtime: JsRuntime,
    pub options: InnerRuntimeOptions,
}
impl InnerRuntime {
    pub fn new(options: InnerRuntimeOptions) -> Result<Self, Error> {
        let loader = Rc::new(RustyLoader::new(options.module_cache));

        // If a snapshot is provided, do not reload ops
        let extensions = if options.startup_snapshot.is_some() {
            ext::all_snapshot_extensions(options.extensions, options.extension_options)
        } else {
            ext::all_extensions(options.extensions, options.extension_options)
        };

        Ok(Self {
            deno_runtime: JsRuntime::try_new(RuntimeOptions {
                module_loader: Some(loader.clone()),

                extension_transpiler: Some(Rc::new(|specifier, code| {
                    transpile_extension(specifier, code)
                })),

                source_map_getter: Some(loader),

                startup_snapshot: options.startup_snapshot,
                extensions,

                ..Default::default()
            })?,

            options: InnerRuntimeOptions {
                timeout: options.timeout,
                default_entrypoint: options.default_entrypoint,
                ..Default::default()
            },
        })
    }

    /// Access the underlying deno runtime instance directly
    pub fn deno_runtime(&mut self) -> &mut JsRuntime {
        &mut self.deno_runtime
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
    /// The function must return a Future that resolves to a serde_json::Value
    /// and accept a vec of serde_json::Value as arguments
    pub fn register_async_function(
        &mut self,
        name: &str,
        callback: RsAsyncFunction,
    ) -> Result<(), Error> {
        self.register_function_generic(name, callback)
    }

    /// Register a rust function
    /// The function must return a serde_json::Value
    /// and accept a slice of serde_json::Value as arguments
    pub fn register_function(&mut self, name: &str, callback: RsFunction) -> Result<(), Error> {
        self.register_function_generic(name, callback)
    }

    /// Register a rust function
    fn register_function_generic<F>(&mut self, name: &str, callback: F) -> Result<(), Error>
    where
        F: 'static,
    {
        let state = self.deno_runtime().op_state();
        let mut state = state.try_borrow_mut()?;

        if !state.has::<HashMap<String, F>>() {
            state.put(HashMap::<String, F>::new());
        }

        // Insert the callback into the state
        state
            .borrow_mut::<HashMap<String, F>>()
            .insert(name.to_string(), callback);

        Ok(())
    }

    /// Get a value from a runtime instance
    ///
    /// # Arguments
    /// * `module_context` - A module handle to use for context, to find exports
    /// * `name` - A string representing the name of the value to find
    ///
    /// # Returns
    /// A `Result` containing the deserialized result or an error (`Error`) if the
    /// value cannot be found, if there are issues with, or if the result cannot be
    /// deserialized.
    pub fn get_value<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let value = self.get_value_ref_async(module_context, name)?;
        let mut scope = self.deno_runtime.handle_scope();
        let value = v8::Local::<v8::Value>::new(&mut scope, value);
        Ok(deno_core::serde_v8::from_v8(&mut scope, value)?)
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
        let result = self.deno_runtime().execute_script("", expr.to_string())?;

        let mut scope = self.deno_runtime.handle_scope();
        let result = v8::Local::new(&mut scope, result);
        Ok(deno_core::serde_v8::from_v8(&mut scope, result)?)
    }

    /// Calls a stored javascript function and deserializes its return value.
    ///
    /// # Arguments
    /// * `module_context` - A module handle to use for context, to find exports
    /// * `function` - A The function object
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function cannot be found, if there are issues with
    /// calling the function, or if the result cannot be deserialized.
    pub fn call_stored_function<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        function: &JsFunction,
        args: &FunctionArguments,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        let function = function.to_v8_global(&mut self.deno_runtime.handle_scope());
        self.call_function_by_ref_async(module_context, function, args)
    }

    /// Calls a javascript function within the Deno runtime by its name and deserializes its return value.
    ///
    /// # Arguments
    /// * `module_context` - A module handle to use for context, to find exports
    /// * `name` - A string representing the name of the javascript function to call.
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function cannot be found, if there are issues with
    /// calling the function, or if the result cannot be deserialized.
    pub fn call_function<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
        args: &FunctionArguments,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        let function = self.get_function_by_name(module_context, name)?;
        self.call_function_by_ref_async(module_context, function, args)
    }

    /// Attempt to get a value out of the global context (globalThis.name)
    ///
    /// # Arguments
    /// * `name` - Name of the object to extract
    ///
    /// # Returns
    /// A `Result` containing the non-null value extracted or an error (`Error`)
    pub fn get_global_value(&mut self, name: &str) -> Result<v8::Global<v8::Value>, Error> {
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
    pub fn get_module_export_value(
        &mut self,
        module_context: &ModuleHandle,
        name: &str,
    ) -> Result<v8::Global<v8::Value>, Error> {
        let module_namespace = self
            .deno_runtime
            .get_module_namespace(module_context.id())?;
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
    pub fn get_value_ref_sync(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
    ) -> Result<v8::Global<v8::Value>, Error> {
        if let Some(module_context) = module_context {
            if let Ok(v) = self.get_module_export_value(module_context, name) {
                return Ok(v);
            }
        }

        self.get_global_value(name)
            .map_err(|_| Error::ValueNotFound(name.to_string()))
    }

    pub fn get_value_ref_async(
        &mut self,
        module_context: Option<&ModuleHandle>,
        name: &str,
    ) -> Result<v8::Global<v8::Value>, Error> {
        let timeout = self.options.timeout;
        Self::run_async_task(
            async move {
                let result = self.get_value_ref_sync(module_context, name)?;
                let future = self.deno_runtime.resolve(result);
                let result = self
                    .deno_runtime
                    .with_event_loop_future(future, Default::default())
                    .await?;

                let mut scope = self.deno_runtime.handle_scope();
                let result = v8::Local::new(&mut scope, result);

                // Decode value
                let value = v8::Global::new(&mut scope, result);
                Ok::<v8::Global<v8::Value>, Error>(value)
            },
            timeout,
        )
    }

    /// This method takes a javascript function and invokes it within the Deno runtime.
    /// It then serializes the return value of the function into a JSON string and
    /// deserializes it into the specified Rust type (`T`).
    ///
    /// # Arguments
    /// * `module_context` - A module handle to use for context, to find exports
    /// * `function` - A reference to a javascript function (`v8::Function`)
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of the function call (`T`)
    /// or an error (`Error`) if the function call fails or the return value cannot
    /// be deserialized.
    pub fn call_function_by_ref_sync(
        &mut self,
        module_context: Option<&ModuleHandle>,
        function: v8::Global<v8::Function>,
        args: &FunctionArguments,
    ) -> Result<v8::Global<v8::Value>, Error> {
        let module_namespace = if let Some(module_context) = module_context {
            Some(
                self.deno_runtime
                    .get_module_namespace(module_context.id())?,
            )
        } else {
            None
        };

        let mut scope = self.deno_runtime.handle_scope();
        let mut scope = v8::TryCatch::new(&mut scope);

        // Get the namespace
        // Module-level if supplied, none otherwise
        let namespace: v8::Local<v8::Value> = match module_namespace {
            Some(namespace) => v8::Local::<v8::Object>::new(&mut scope, namespace).into(),
            None => {
                // Create a new object to use as the namespace if none is provided
                //let obj: v8::Local<v8::Value> = v8::Object::new(&mut scope).into();
                let obj: v8::Local<v8::Value> = v8::undefined(&mut scope).into();
                obj
            }
        };

        let function_instance = function.open(&mut scope);

        // Prep argumentsgit
        let f_args: Result<Vec<v8::Local<v8::Value>>, deno_core::serde_v8::Error> = args
            .iter()
            .map(|f| deno_core::serde_v8::to_v8(&mut scope, f))
            .collect();
        let final_args = f_args?;

        let result = function_instance.call(&mut scope, namespace, &final_args);
        match result {
            Some(value) => {
                let value = v8::Global::new(&mut scope, value);
                Ok(value)
            }
            None if scope.has_caught() => {
                let e = scope.message().unwrap();

                let filename = e.get_script_resource_name(&mut scope);
                let linenumber = e.get_line_number(&mut scope).unwrap_or_default();
                let filename = if let Some(v) = filename {
                    let filename = v.to_rust_string_lossy(&mut scope);
                    format!("{filename}:{linenumber}: ")
                } else if let Some(module_context) = module_context {
                    let filename = module_context.module().filename().to_string();
                    format!("{filename}:{linenumber}: ")
                } else {
                    "".to_string()
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
        let value = self.get_value_ref_sync(module_context, name)?;

        // Convert it into a function
        let mut scope = self.deno_runtime.handle_scope();
        let local_value = v8::Local::<v8::Value>::new(&mut scope, value);
        let f: v8::Local<v8::Function> = local_value
            .try_into()
            .or::<Error>(Err(Error::ValueNotCallable(name.to_string())))?;

        // Return it as a global
        Ok(v8::Global::<v8::Function>::new(&mut scope, f))
    }

    pub fn call_function_by_ref_async<T>(
        &mut self,
        module_context: Option<&ModuleHandle>,
        function: v8::Global<v8::Function>,
        args: &FunctionArguments,
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        let timeout = self.options.timeout;
        Self::run_async_task(
            async move {
                let result = self.call_function_by_ref_sync(module_context, function, args)?;
                let future = self.deno_runtime.resolve(result);
                let result = self
                    .deno_runtime
                    .with_event_loop_future(future, Default::default())
                    .await?;

                //let result = self.deno_runtime.resolve(result).await?;

                let mut scope = self.deno_runtime.handle_scope();
                let result = v8::Local::new(&mut scope, result);

                // Decode value
                let value: T = deno_core::serde_v8::from_v8(&mut scope, result)?;
                Ok::<T, Error>(value)
            },
            timeout,
        )
    }

    pub fn run_async_task<T, F>(f: F, timeout: Duration) -> Result<T, Error>
    where
        F: tokio::macros::support::Future + std::future::Future<Output = Result<T, Error>>,
    {
        let tokio_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .thread_keep_alive(timeout)
            .build()?;

        tokio_runtime.block_on(async move {
            let _f = tokio::time::timeout(timeout, f);
            _f.await
        })?
    }

    /// Load one or more modules
    ///
    /// Will return a handle to the main module, or the last
    /// side-module
    pub fn load_modules(
        &mut self,
        main_module: Option<&Module>,
        side_modules: Vec<&Module>,
    ) -> Result<ModuleHandle, Error> {
        let timeout = self.options.timeout;
        let default_entrypoint = self.options.default_entrypoint.clone();

        if main_module.is_none() && side_modules.is_empty() {
            return Err(Error::Runtime(
                "Internal error: attempt to load no modules".to_string(),
            ));
        }

        let deno_runtime = &mut self.deno_runtime();
        let module_handle_stub = Self::run_async_task(
            async move {
                let mut module_handle_stub = Default::default();

                // Get additional modules first
                for side_module in side_modules {
                    let module_specifier = side_module.filename().to_module_specifier()?;
                    let (code, _) =
                        transpiler::transpile(&module_specifier, side_module.contents())?;
                    let code = deno_core::FastString::from(code);

                    let s_modid = deno_runtime
                        .load_side_es_module_from_code(&module_specifier, code)
                        .await?;
                    let result = deno_runtime.mod_evaluate(s_modid);
                    deno_runtime
                        .run_event_loop(PollEventLoopOptions::default())
                        .await?;
                    result.await?;
                    module_handle_stub = ModuleHandle::new(side_module, s_modid, None);
                }

                // Load main module
                if let Some(module) = main_module {
                    let module_specifier = module.filename().to_module_specifier()?;
                    let (code, _) = transpiler::transpile(&module_specifier, module.contents())?;
                    let code = deno_core::FastString::from(code);

                    let module_id = deno_runtime
                        .load_main_es_module_from_code(&module_specifier, code)
                        .await?;

                    // Finish execution
                    let result = deno_runtime.mod_evaluate(module_id);
                    deno_runtime
                        .run_event_loop(PollEventLoopOptions {
                            wait_for_inspector: false,
                            ..Default::default()
                        })
                        .await?;
                    result.await?;
                    module_handle_stub = ModuleHandle::new(module, module_id, None);
                }

                Ok::<ModuleHandle, Error>(module_handle_stub)
            },
            timeout,
        )?;

        // Try to get an entrypoint
        let state = self.deno_runtime().op_state();
        let mut deep_state = state.try_borrow_mut()?;
        let f_entrypoint = match deep_state.try_take::<v8::Global<v8::Function>>() {
            Some(entrypoint) => Some(entrypoint),
            None => default_entrypoint.and_then(|default_entrypoint| {
                self.get_function_by_name(Some(&module_handle_stub), &default_entrypoint)
                    .ok()
            }),
        };

        Ok(ModuleHandle::new(
            module_handle_stub.module(),
            module_handle_stub.id(),
            f_entrypoint,
        ))
    }
}

#[cfg(test)]
mod test_inner_runtime {
    use serde::Deserialize;

    use super::*;
    use crate::{json_args, Undefined};

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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
        let module = runtime
            .load_modules(Some(&module), vec![])
            .expect("Could not load module");

        assert_eq!(
            2,
            runtime
                .get_value::<usize>(Some(&module), "a")
                .expect("Could not find global")
        );
        assert_eq!(
            "test",
            runtime
                .get_value::<String>(Some(&module), "b")
                .expect("Could not find export")
        );
        runtime
            .get_value::<Undefined>(Some(&module), "c")
            .expect_err("Could not detect null");
        runtime
            .get_value::<Undefined>(Some(&module), "d")
            .expect_err("Could not detect undeclared");
    }

    #[test]
    fn test_get_value_by_ref() {
        let module = Module::new(
            "test.js",
            "
            globalThis.a = 2;
            export const b = 'test';
            export const fnc = null;
        ",
        );

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
        let module = runtime
            .load_modules(Some(&module), vec![])
            .expect("Could not load module");

        runtime
            .get_value_ref_async(Some(&module), "a")
            .expect("Could not find global");
        runtime
            .get_value_ref_async(Some(&module), "b")
            .expect("Could not find export");
        runtime
            .get_value_ref_async(Some(&module), "c")
            .expect_err("Could not detect null");
        runtime
            .get_value_ref_async(Some(&module), "d")
            .expect_err("Could not detect undeclared");
    }

    #[test]
    fn call_function() {
        let module = Module::new(
            "test.js",
            "
            globalThis.fna = (i) => i;
            export function fnb() { 
                return 'test'; 
            }
            export const fnc = 2;
            export const fne = () => {};
        ",
        );

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
        let module = runtime
            .load_modules(Some(&module), vec![])
            .expect("Could not load module");

        let result: usize = runtime
            .call_function(Some(&module), "fna", json_args!(2))
            .expect("Could not call global");
        assert_eq!(2, result);

        let result: String = runtime
            .call_function(Some(&module), "fnb", json_args!())
            .expect("Could not call export");
        assert_eq!("test", result);

        runtime
            .call_function::<Undefined>(Some(&module), "fnc", json_args!())
            .expect_err("Did not detect non-function");
        runtime
            .call_function::<Undefined>(Some(&module), "fnd", json_args!())
            .expect_err("Did not detect undefined");
        runtime
            .call_function::<Undefined>(Some(&module), "fne", json_args!())
            .expect("Did not allow undefined return");
    }

    #[test]
    fn call_errorfunction() {
        let module = Module::new(
            "test.js",
            "
            export const fn = () => { throw new Error('msg') };
        ",
        );

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
        let module = runtime
            .load_modules(Some(&module), vec![])
            .expect("Could not load module");

        let e = runtime
            .call_function::<usize>(Some(&module), "fn", json_args!(1))
            .unwrap_err();
        assert!(e.to_string().ends_with("test.js:2: Uncaught Error: msg"));
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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
        let module = runtime
            .load_modules(Some(&module), vec![])
            .expect("Could not load module");

        let result: usize = runtime
            .call_function(Some(&module), "test", json_args!(2, 3))
            .expect("Could not call global");
        assert_eq!(5, result);
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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
        let module = runtime
            .load_modules(Some(&module), vec![])
            .expect("Could not load module");

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

    #[cfg(feature = "web")]
    #[test]
    fn test_tla() {
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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
        let module = runtime
            .load_modules(Some(&module), vec![])
            .expect("Could not load module");

        let value: usize = runtime
            .call_function(Some(&module), "test", json_args!())
            .expect("Could not call function");
        assert_eq!(value, 2);
    }

    #[cfg(feature = "web")]
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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
        let module = runtime
            .load_modules(Some(&module), vec![])
            .expect("Could not load module");

        let value: usize = runtime
            .call_function(Some(&module), "test", json_args!())
            .expect("Could not call function");
        assert_eq!(value, 2);
    }

    #[cfg(feature = "web")]
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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
        let module = runtime
            .load_modules(Some(&module), vec![])
            .expect("Could not load module");

        let value: usize = runtime
            .call_function(Some(&module), "test", json_args!())
            .expect("Could not call function");
        assert_eq!(value, 2);
    }

    #[test]
    fn test_serialize_deep_fn() {
        let module = Module::new(
            "test.js",
            "
            export const test = {
                'name': 'test',
                'func': (x) => 3*x+1
            }
        ",
        );

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
        let module = runtime
            .load_modules(Some(&module), vec![])
            .expect("Could not load module");

        #[derive(Deserialize)]
        struct TestStruct<'a> {
            #[allow(dead_code)]
            name: String,
            func: JsFunction<'a>,
        }
        let structure: TestStruct = runtime
            .get_value(Some(&module), "test")
            .expect("Could not get object");

        let value: usize = runtime
            .call_stored_function(Some(&module), &structure.func, json_args!(2))
            .expect("could not call function");
        assert_eq!(7, value);

        let value: usize = runtime
            .call_stored_function(None, &structure.func, json_args!(2))
            .expect("could not call function");
        assert_eq!(7, value);
    }

    #[test]
    fn test_serialize_fn() {
        let module = Module::new(
            "test.js",
            "
            export const test = (x) => 2*x;
        ",
        );

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
        let module = runtime
            .load_modules(Some(&module), vec![])
            .expect("Could not load module");

        let function: JsFunction = runtime
            .get_value(Some(&module), "test")
            .expect("Could not get function");

        println!("Deserialized");
        let value: usize = runtime
            .call_stored_function(Some(&module), &function, json_args!(2))
            .expect("could not call function");
        assert_eq!(4, value);
    }
}
