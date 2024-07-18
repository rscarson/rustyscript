use super::V8Value;
use deno_core::v8::{self, HandleScope};
use serde::Deserialize;

/// A Deserializable javascript function, that can be stored and used later
/// Must live as long as the runtime it was birthed from
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Function(V8Value<FunctionTypeChecker>);
impl_v8!(Function, FunctionTypeChecker);
impl_checker!(FunctionTypeChecker, Function, is_function, |e| {
    crate::Error::ValueNotCallable(e)
});

impl Function {
    pub(crate) fn as_global(&self, scope: &mut HandleScope<'_>) -> v8::Global<v8::Function> {
        self.0.as_global(scope)
    }

    /// Returns true if the function is async
    #[must_use]
    pub fn is_async(&self) -> bool {
        // Safe because we aren't applying this to an isolate
        let unsafe_f = unsafe { v8::Handle::get_unchecked(&self.0 .0) };
        unsafe_f.is_async_function()
    }

    /// Calls this function. See [`crate::Runtime::call_stored_function`]
    /// Blocks until:
    /// - The event loop is resolved, and
    /// - If the value is a promise, the promise is resolved
    ///
    /// # Errors
    /// Will return an error if the function cannot be called, if the function returns an error
    /// Or if the function returns a value that cannot be deserialized into the given type
    pub fn call<T>(
        &self,
        runtime: &mut crate::Runtime,
        module_context: Option<&crate::ModuleHandle>,
        args: &impl serde::ser::Serialize,
    ) -> Result<T, crate::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        runtime.call_stored_function(module_context, self, args)
    }

    /// Calls this function. See [`crate::Runtime::call_stored_function_async`]
    /// Returns a future that resolves when:
    /// - The event loop is resolved, and
    /// - If the value is a promise, the promise is resolved
    ///
    /// # Errors
    /// Will return an error if the function cannot be called, if the function returns an error
    /// Or if the function returns a value that cannot be deserialized into the given type
    pub async fn call_async<T>(
        &self,
        runtime: &mut crate::Runtime,
        module_context: Option<&crate::ModuleHandle>,
        args: &impl serde::ser::Serialize,
    ) -> Result<T, crate::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        runtime
            .call_stored_function_async(module_context, self, args)
            .await
    }

    /// Calls this function. See [`crate::Runtime::call_stored_function_immediate`]
    /// Does not wait for the event loop to resolve, or attempt to resolve promises
    ///
    /// # Errors
    /// Will return an error if the function cannot be called, if the function returns an error
    /// Or if the function returns a value that cannot be deserialized into the given type
    pub fn call_immediate<T>(
        &self,
        runtime: &mut crate::Runtime,
        module_context: Option<&crate::ModuleHandle>,
        args: &impl serde::ser::Serialize,
    ) -> Result<T, crate::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        runtime.call_stored_function_immediate(module_context, self, args)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{js_value::Promise, json_args, Module, Runtime, RuntimeOptions};

    #[test]
    fn test_function() {
        let module = Module::new(
            "test.js",
            "
            export const f = () => 42;
            export const f2 = async () => 42;
        ",
        );

        let mut runtime = Runtime::new(RuntimeOptions::default()).unwrap();
        let handle = runtime.load_module(&module).unwrap();

        let f: Function = runtime.get_value(Some(&handle), "f").unwrap();
        let value: usize = f.call(&mut runtime, Some(&handle), &json_args!()).unwrap();
        assert_eq!(value, 42);

        let f2: Function = runtime.get_value(Some(&handle), "f2").unwrap();
        let value: Promise<usize> = f2
            .call_immediate(&mut runtime, Some(&handle), &json_args!())
            .unwrap();
        let value = value.into_value(&mut runtime).unwrap();
        assert_eq!(value, 42);
    }
}
