use crate::{
    cache_provider::ModuleCacheProvider,
    ext,
    module_loader::RustyLoader,
    traits::{ToDefinedValue, ToModuleSpecifier, ToV8String},
    transpiler::{self, transpile_extension},
    Error, Module, ModuleHandle,
};
use deno_core::{
    serde_json, serde_v8::from_v8, v8, JsRuntime, PollEventLoopOptions, RuntimeOptions,
};
use serde::de::DeserializeOwned;
use std::{collections::HashMap, pin::Pin, rc::Rc, time::Duration};

/// Represents a function that can be registered with the runtime
pub trait RsFunction: Fn(&FunctionArguments) -> Result<serde_json::Value, Error> + 'static {}
impl<F> RsFunction for F where
    F: Fn(&FunctionArguments) -> Result<serde_json::Value, Error> + 'static
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

/// Type required to pass arguments to Functions
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

    /// Optional configuration parameters for building the underlying v8 isolate
    /// This can be used to alter the behavior of the runtime.
    /// See the rusty_v8 documentation for more information
    pub isolate_params: Option<v8::CreateParams>,
}

impl Default for InnerRuntimeOptions {
    fn default() -> Self {
        Self {
            extensions: Default::default(),
            default_entrypoint: Default::default(),
            timeout: Duration::MAX,
            module_cache: None,
            startup_snapshot: None,
            isolate_params: None,

            extension_options: Default::default(),
        }
    }
}

/// Deno JsRuntime wrapper providing helper functions needed
/// by the public-facing Runtime API
///
/// This struct is not intended to be used directly by the end user
/// It provides a set of async functions that can be used to interact with the
/// underlying deno runtime instance
pub struct InnerRuntime {
    pub module_loader: Rc<RustyLoader>,
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

                source_map_getter: Some(loader.clone()),
                create_params: options.isolate_params,

                startup_snapshot: options.startup_snapshot,
                extensions,

                ..Default::default()
            })?,

            module_loader: loader,

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
    /// The function must return a serde_json::Value
    /// and accept a slice of serde_json::Value as arguments
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
    pub async fn await_event_loop(&mut self, options: PollEventLoopOptions) -> Result<(), Error> {
        Ok(self.deno_runtime.run_event_loop(options).await?)
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
        T: DeserializeOwned,
    {
        // Update source map cache
        self.module_loader
            .insert_source_map("", expr.to_string(), None);

        let result = self.deno_runtime().execute_script("", expr.to_string())?;

        let mut scope = self.deno_runtime.handle_scope();
        let result = v8::Local::new(&mut scope, result);
        Ok(from_v8(&mut scope, result)?)
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

    /// Attempt to get a value out of a module context (export ``...)
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

    pub async fn resolve_with_event_loop(
        &mut self,
        value: v8::Global<v8::Value>,
    ) -> Result<v8::Global<v8::Value>, Error> {
        let future = self.deno_runtime.resolve(value);
        let result = self
            .deno_runtime
            .with_event_loop_future(future, Default::default())
            .await?;
        Ok(result)
    }

    pub fn decode_value<T>(&mut self, value: v8::Global<v8::Value>) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let mut scope = self.deno_runtime.handle_scope();
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
        let mut scope = self.deno_runtime.handle_scope();
        let local_value = v8::Local::<v8::Value>::new(&mut scope, value);
        let f: v8::Local<v8::Function> = local_value
            .try_into()
            .or::<Error>(Err(Error::ValueNotCallable(name.to_string())))?;

        // Return it as a global
        Ok(v8::Global::<v8::Function>::new(&mut scope, f))
    }

    pub async fn call_function_by_ref(
        &mut self,
        module_context: Option<&ModuleHandle>,
        function: v8::Global<v8::Function>,
        args: &FunctionArguments,
    ) -> Result<v8::Global<v8::Value>, Error> {
        // Namespace, if provided
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

        // Prep argument
        let f_args: Result<Vec<v8::Local<v8::Value>>, deno_core::serde_v8::Error> = args
            .iter()
            .map(|f| deno_core::serde_v8::to_v8(&mut scope, f))
            .collect();
        let final_args = f_args?;

        // Call the function
        let result = function_instance.call(&mut scope, namespace, &final_args);
        match result {
            Some(value) => {
                let value = v8::Global::new(&mut scope, value);
                Ok(value)
            }
            None if scope.has_caught() => {
                let e = match scope.message() {
                    Some(e) => e,
                    None => return Err(Error::Runtime("Unknown error".to_string())),
                };

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
        let default_entrypoint = self.options.default_entrypoint.clone();

        if main_module.is_none() && side_modules.is_empty() {
            return Err(Error::Runtime(
                "Internal error: attempt to load no modules".to_string(),
            ));
        }

        let mut module_handle_stub = Default::default();

        // Get additional modules first
        for side_module in side_modules {
            let module_specifier = side_module.filename().to_module_specifier()?;
            let (code, sourcemap) =
                transpiler::transpile(&module_specifier, side_module.contents())?;
            let fast_code = deno_core::FastString::from(code.clone());

            let s_modid = self
                .deno_runtime
                .load_side_es_module_from_code(&module_specifier, fast_code)
                .await?;

            // Update source map cache
            self.module_loader.insert_source_map(
                module_specifier.as_str(),
                code,
                sourcemap.map(|s| s.to_vec()),
            );

            let result = self.deno_runtime.mod_evaluate(s_modid);
            self.deno_runtime
                .run_event_loop(PollEventLoopOptions::default())
                .await?;
            result.await?;
            module_handle_stub = ModuleHandle::new(side_module, s_modid, None);
        }

        // Load main module
        if let Some(module) = main_module {
            let module_specifier = module.filename().to_module_specifier()?;
            let (code, sourcemap) = transpiler::transpile(&module_specifier, module.contents())?;
            let fast_code = deno_core::FastString::from(code.clone());

            let module_id = self
                .deno_runtime
                .load_main_es_module_from_code(&module_specifier, fast_code)
                .await?;

            // Update source map cache
            self.module_loader.insert_source_map(
                module_specifier.as_str(),
                code,
                sourcemap.map(|s| s.to_vec()),
            );

            // Finish execution
            let result = self.deno_runtime.mod_evaluate(module_id);
            self.deno_runtime
                .run_event_loop(PollEventLoopOptions {
                    wait_for_inspector: false,
                    ..Default::default()
                })
                .await?;
            result.await?;
            module_handle_stub = ModuleHandle::new(module, module_id, None);
        }

        // Try to get an entrypoint
        let state = self.deno_runtime().op_state();
        let mut deep_state = state.try_borrow_mut()?;
        let f_entrypoint = match deep_state.try_take::<v8::Global<v8::Function>>() {
            Some(entrypoint) => Some(entrypoint),
            None => match default_entrypoint {
                None => None,
                Some(entrypoint) => self
                    .get_function_by_name(Some(&module_handle_stub), &entrypoint)
                    .ok(),
            },
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

    use crate::{async_callback, js_value::Function, json_args, sync_callback};

    #[cfg(any(feature = "web", feature = "web_stub"))]
    use crate::js_value::Promise;

    use super::*;

    /// Used for blocking functions
    fn run_async_task<'a, T, F, U>(f: F) -> T
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
    fn test_put_take() {
        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");

        runtime.put(2usize).expect("Could not put value");
        let v = runtime.take::<usize>().expect("Could not take value");
        assert_eq!(v, 2);
    }

    #[test]
    fn test_register_async_function() {
        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
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
        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");
        runtime
            .register_function(
                "test",
                sync_callback!(|a: i64, b: i64| { Ok::<i64, Error>(a + b) }),
            )
            .expect("Could not register function");

        let result: i64 = runtime
            .eval("rustyscript.functions.test(2, 3)")
            .expect("Could not eval");
        assert_eq!(result, 5);
    }

    #[cfg(any(feature = "web", feature = "web_stub"))]
    #[test]
    fn test_eval() {
        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");

        let result: usize = runtime.eval("2 + 2").expect("Could not eval");
        assert_eq!(result, 4);

        run_async_task(|| async move {
            let result: Promise<usize> = runtime
                .eval(
                    "
                let sleep = (ms) => new Promise((r) => setTimeout(r, ms));
                sleep(500).then(() => 2);
            ",
                )
                .expect("Could not eval");

            let result: usize = result.resolve(&mut runtime.deno_runtime()).await?;
            assert_eq!(result, 2);

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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");

        let rt = &mut runtime;
        let module = run_async_task(|| async move { rt.load_modules(Some(&module), vec![]).await });

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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");

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
                InnerRuntime::new(Default::default()).expect("Could not load runtime");
            let handle = runtime.load_modules(Some(&module), vec![]).await?;

            let f = runtime.get_function_by_name(None, "fna").unwrap();
            let result = runtime
                .call_function_by_ref(Some(&handle), f, json_args!(2))
                .await
                .expect("Could not call global");
            assert_v8!(result, 2, usize, runtime);

            let f = runtime.get_function_by_name(Some(&handle), "fnb").unwrap();
            let result = runtime
                .call_function_by_ref(Some(&handle), f, json_args!())
                .await
                .expect("Could not call export");
            assert_v8!(result, "test", String, runtime);

            let f = runtime.get_function_by_name(Some(&handle), "fne").unwrap();
            runtime
                .call_function_by_ref(Some(&handle), f, json_args!())
                .await
                .expect("Did not allow undefined return");

            let f = runtime
                .get_function_by_name(Some(&handle), "will_err")
                .unwrap();
            runtime
                .call_function_by_ref(Some(&handle), f, json_args!())
                .await
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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");

        let rt = &mut runtime;
        let module = run_async_task(|| async move { rt.load_modules(Some(&module), vec![]).await });

        let f = runtime.get_function_by_name(Some(&module), "test").unwrap();
        let rt = &mut runtime;
        let result = run_async_task(|| async move {
            rt.call_function_by_ref(Some(&module), f, json_args!(2, 3))
                .await
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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");

        let rt = &mut runtime;
        let module = run_async_task(|| async move { rt.load_modules(Some(&module), vec![]).await });

        let f = runtime.get_function_by_name(Some(&module), "test").unwrap();
        let rt = &mut runtime;
        let result = run_async_task(|| async move {
            rt.call_function_by_ref(Some(&module), f, json_args!())
                .await
        });
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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");

        let rt = &mut runtime;
        run_async_task(|| async move {
            let module = rt.load_modules(Some(&module), vec![]).await?;

            let f = rt.get_function_by_name(Some(&module), "test").unwrap();
            let result = rt
                .call_function_by_ref(Some(&module), f, json_args!())
                .await?;

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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");

        let rt = &mut runtime;
        run_async_task(|| async move {
            let module = rt.load_modules(Some(&module), vec![]).await?;

            let f = rt.get_function_by_name(Some(&module), "test")?;
            let result = rt
                .call_function_by_ref(Some(&module), f, json_args!())
                .await?;
            let result: Promise<usize> = rt.decode_value(result).expect("Could not deserialize");
            let result: usize = result.resolve(&mut rt.deno_runtime()).await?;
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

        let mut runtime = InnerRuntime::new(Default::default()).expect("Could not load runtime");

        let rt = &mut runtime;
        let module = run_async_task(|| async move { rt.load_modules(Some(&module), vec![]).await });

        #[derive(Deserialize)]
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
                .call_function_by_ref(Some(&module), function.clone(), json_args!(2))
                .await
                .expect("could not call function");
            assert_v8!(value, 4, usize, runtime);

            let value = runtime
                .call_function_by_ref(Some(&module), function, json_args!(3))
                .await
                .expect("could not call function twice");
            assert_v8!(value, 5, usize, runtime);

            Ok(())
        });
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

        let rt = &mut runtime;
        let module = run_async_task(|| async move { rt.load_modules(Some(&module), vec![]).await });

        let function = runtime
            .get_function_by_name(Some(&module), "test")
            .expect("Could not get function");

        run_async_task(|| async move {
            let value = runtime
                .call_function_by_ref(Some(&module), function.clone(), json_args!(2))
                .await
                .expect("could not call function");
            assert_v8!(value, 4, usize, runtime);

            let value = runtime
                .call_function_by_ref(None, function, json_args!(2))
                .await
                .expect("could not call function");
            assert_v8!(value, 4, usize, runtime);

            Ok(())
        });
    }
}
