use std::{rc::Rc, time::Duration};

use deno_core::{serde_json, v8, Extension, FsModuleLoader, JsRuntime, RuntimeOptions};

use crate::{
    ext::js_playground,
    traits::{ToDefinedValue, ToModuleSpecifier, ToV8String},
    Error, ModuleHandle, Script,
};

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

/// Deno JsRuntime wrapper providing helper functions needed
/// by the public-facing Runtime API
pub struct InnerRuntime(JsRuntime);
impl InnerRuntime {
    pub fn new(extensions: Vec<Extension>) -> Self {
        Self(JsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(FsModuleLoader)),
            extensions: Self::all_extensions(extensions),
            ..Default::default()
        }))
    }

    /// Access the underlying deno runtime instance directly
    pub fn deno_runtime(&mut self) -> &mut JsRuntime {
        &mut self.0
    }

    pub fn clear_modules(&mut self) {
        todo!("self.0.clear_modules()")
    }

    /// Get a value from a runtime instance
    ///
    /// # Arguments
    /// * `name` - A string representing the name of the value to find
    ///
    /// # Returns
    /// A `Result` containing the deserialized result or an error (`Error`) if the
    /// value cannot be found, if there are issues with, or if the result cannot be
    /// deserialized.
    pub fn get_value<T>(&mut self, module_context: &ModuleHandle, name: &str) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        let value = self.get_value_ref(module_context, name)?;
        let mut scope = self.0.handle_scope();
        let local_value = v8::Local::<v8::Value>::new(&mut scope, value);
        Ok(deno_core::serde_v8::from_v8(&mut scope, local_value)?)
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

    /// Attempt to get a value out of the global context (globalThis.name)
    ///
    /// # Arguments
    /// * `name` - Name of the object to extract
    ///
    /// # Returns
    /// A `Result` containing the non-null value extracted or an error (`Error`)
    pub fn get_global_value(&mut self, name: &str) -> Result<v8::Global<v8::Value>, Error> {
        let context = self.0.main_context();
        let mut scope = self.0.handle_scope();
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
        module: &ModuleHandle,
        name: &str,
    ) -> Result<v8::Global<v8::Value>, Error> {
        let module_namespace = self.0.get_module_namespace(module.id())?;
        let mut scope = self.0.handle_scope();
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
    pub fn get_value_ref(
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
    pub fn call_function_by_ref<T>(
        &mut self,
        module_context: &ModuleHandle,
        function: v8::Global<v8::Function>,
        args: &[serde_json::Value],
    ) -> Result<T, Error>
    where
        T: deno_core::serde::de::DeserializeOwned,
    {
        let module_namespace = self.0.get_module_namespace(module_context.id())?;
        let mut scope = self.0.handle_scope();
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
    pub fn get_function_by_name(
        &mut self,
        module_context: &ModuleHandle,
        name: &str,
    ) -> Result<v8::Global<v8::Function>, Error> {
        // Get the value
        let value = self.get_value_ref(module_context, name)?;

        // Convert it into a function
        let mut scope = self.0.handle_scope();
        let local_value = v8::Local::<v8::Value>::new(&mut scope, value);
        let f: v8::Local<v8::Function> = local_value
            .try_into()
            .or::<Error>(Err(Error::ValueNotCallable(name.to_string())))?;

        // Return it as a global
        Ok(v8::Global::<v8::Function>::new(&mut scope, f))
    }

    /// Load one or more modules
    ///
    /// Will return a handle to the main module, or the last
    /// side-module
    pub fn load_modules(
        &mut self,
        main_module: Option<&Script>,
        side_modules: Vec<&Script>,
        timeout: Option<Duration>,
        default_entrypoint: Option<String>,
    ) -> Result<ModuleHandle, Error> {
        if main_module.is_none() && side_modules.is_empty() {
            return Err(Error::Runtime(
                "Internal error: attempt to load no modules".to_string(),
            ));
        }

        // Evaluate the script
        let deno_runtime = &mut self.deno_runtime();
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
        let tokio_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        // Let it play out...
        let module_handle_stub = tokio_runtime.block_on(future)?;
        if let Some(timeout) = timeout {
            tokio_runtime.shutdown_timeout(timeout);
        }

        // Try to get an entrypoint
        let state = self.deno_runtime().op_state();
        let mut deep_state = state.try_borrow_mut()?;
        let f_entrypoint = match deep_state.try_take::<v8::Global<v8::Function>>() {
            Some(entrypoint) => Some(entrypoint),
            None => default_entrypoint.and_then(|default_entrypoint| {
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

    /// Adds internal extensions to the list provided by the user
    ///
    /// # Arguments
    /// * `user_extensions` - A set of deno_core::Extension objects provided by the user
    fn all_extensions(mut user_extensions: Vec<Extension>) -> Vec<Extension> {
        user_extensions.extend(vec![js_playground::js_playground::init_ops_and_esm()]);

        #[cfg(feature = "console")]
        user_extensions.extend(vec![
            deno_console::deno_console::init_ops_and_esm(),
            crate::ext::init_console::init_console::init_ops_and_esm(),
        ]);

        #[cfg(feature = "url")]
        user_extensions.extend(vec![
            deno_webidl::deno_webidl::init_ops_and_esm(),
            deno_url::deno_url::init_ops_and_esm(),
            crate::ext::init_url::init_url::init_ops_and_esm(),
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
mod test_inner_runtime {
    use super::*;
    use crate::{Runtime, Undefined};

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

        let mut runtime = InnerRuntime::new(vec![]);
        let module = runtime
            .load_modules(Some(&script), vec![], None, None)
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
    fn test_get_value_by_ref() {
        let script = Script::new(
            "test.js",
            "
            globalThis.a = 2;
            export const b = 'test';
            export const fnc = null;
        ",
        );

        let mut runtime = InnerRuntime::new(vec![]);
        let module = runtime
            .load_modules(Some(&script), vec![], None, None)
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

        let mut runtime = InnerRuntime::new(vec![]);
        let module = runtime
            .load_modules(Some(&script), vec![], None, None)
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

        let mut runtime = InnerRuntime::new(vec![]);
        let module = runtime
            .load_modules(Some(&script), vec![], None, None)
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
