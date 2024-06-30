//! This module a
use deno_core::v8::{self, HandleScope};
use serde::Deserialize;

/// A trait that is used to check if a `v8::Value` is of a certain type
trait V8TypeChecker {
    /// Converts a `v8::Global<v8::Value>` to a `&v8::Value`
    ///
    /// # Safety
    /// non-existant. Do not call this function
    fn into_raw(value: &v8::Global<v8::Value>) -> &v8::Value {
        unsafe { v8::Handle::get_unchecked(value) }
    }

    /// Checks if a `v8::Value` is of a certain type
    fn is_valid(value: v8::Global<v8::Value>) -> bool;
}

/// Implementations of `V8TypeChecker` for functions
#[derive(Eq, Hash, PartialEq, Debug, Clone, Deserialize)]
struct FunctionTypeChecker;
impl V8TypeChecker for FunctionTypeChecker {
    fn is_valid(value: v8::Global<v8::Value>) -> bool {
        Self::into_raw(&value).is_function()
    }
}

/// Implementations of `V8TypeChecker` for promises
#[derive(Eq, Hash, PartialEq, Debug, Clone, Deserialize)]
struct PromiseTypeChecker;
impl V8TypeChecker for PromiseTypeChecker {
    fn is_valid(value: v8::Global<v8::Value>) -> bool {
        Self::into_raw(&value).is_promise()
    }
}

/// Implementations of `V8TypeChecker` for any value
#[derive(Eq, Hash, PartialEq, Debug, Clone, Deserialize)]
struct DefaultTypeChecker;
impl V8TypeChecker for DefaultTypeChecker {
    fn is_valid(_: v8::Global<v8::Value>) -> bool {
        true
    }
}

/// A Deserializable javascript object, that can be stored and used later
/// Must live as long as the runtime it was birthed from
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
struct V8Value<V8TypeChecker>(
    v8::Global<v8::Value>,
    std::marker::PhantomData<V8TypeChecker>,
);

impl<T> V8Value<T> {
    pub(crate) fn into_local<'a, V>(&self, scope: &mut HandleScope<'a>) -> Option<v8::Local<'a, V>>
    where
        v8::Local<'a, V>: TryFrom<v8::Local<'a, v8::Value>>,
    {
        let local = v8::Local::new(scope, &self.0);
        v8::Local::<'a, V>::try_from(local).ok()
    }

    pub(crate) fn into_global<'a, V>(&self, scope: &mut HandleScope<'a>) -> Option<v8::Global<V>>
    where
        v8::Local<'a, V>: TryFrom<v8::Local<'a, v8::Value>>,
    {
        let local = self.into_local(scope)?;
        Some(v8::Global::new(scope, local))
    }
}

impl<'de, T: V8TypeChecker> serde::Deserialize<'de> for V8Value<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = crate::v8_serializer::GlobalValue::deserialize(deserializer)?;
        if T::is_valid(value.v8_value.clone()) {
            Ok(Self(value.v8_value, std::marker::PhantomData))
        } else {
            Err(serde::de::Error::custom("Invalid V8 value"))
        }
    }
}

/// A Deserializable javascript function, that can be stored and used later
/// Must live as long as the runtime it was birthed from
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Function(V8Value<FunctionTypeChecker>);
impl Function {
    pub(crate) fn into_global<'a>(
        &self,
        scope: &mut HandleScope<'a>,
    ) -> Result<v8::Global<v8::Function>, crate::Error> {
        self.0
            .into_global(scope)
            .ok_or_else(|| crate::Error::ValueNotCallable("function".to_string()))
    }

    /// Calls this function. See [Runtime::call_stored_function]
    /// Blocks until:
    /// - The event loop is resolved, and
    /// - If the value is a promise, the promise is resolved
    pub fn call<T>(
        &self,
        runtime: &mut crate::Runtime,
        module_context: Option<&crate::ModuleHandle>,
        args: &crate::FunctionArguments,
    ) -> Result<T, crate::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        runtime.call_stored_function(module_context, self, args)
    }

    /// Calls this function. See [Runtime::call_stored_function_async]
    /// Returns a future that resolves when:
    /// - The event loop is resolved, and
    /// - If the value is a promise, the promise is resolved
    pub async fn call_async<T>(
        &self,
        runtime: &mut crate::Runtime,
        module_context: Option<&crate::ModuleHandle>,
        args: &crate::FunctionArguments,
    ) -> Result<T, crate::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        runtime
            .call_stored_function_async(module_context, self, args)
            .await
    }

    /// Calls this function. See [Runtime::call_stored_function_immediate]
    /// Does not wait for the event loop to resolve, or attempt to resolve promises
    pub fn call_immediate<T>(
        &self,
        runtime: &mut crate::Runtime,
        module_context: Option<&crate::ModuleHandle>,
        args: &crate::FunctionArguments,
    ) -> Result<T, crate::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        runtime.call_stored_function_immediate(module_context, self, args)
    }

    /// Returns the underlying v8 value
    pub fn into_v8(self) -> v8::Global<v8::Value> {
        self.0 .0
    }
}
impl<'de> serde::Deserialize<'de> for Function {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = V8Value::<FunctionTypeChecker>::deserialize(deserializer)?;
        Ok(Self(inner))
    }
}

/// A Deserializable javascript promise, that can be stored and used later
/// Must live as long as the runtime it was birthed from
///
/// You can turn `Promise<T>` into `Future<Output = T>` by calling `Promise::into_future`
/// This allows you to export multiple concurrent promises without borrowing the runtime mutably
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Promise<T>(V8Value<PromiseTypeChecker>, std::marker::PhantomData<T>)
where
    T: serde::de::DeserializeOwned;
impl<T> Promise<T>
where
    T: serde::de::DeserializeOwned,
{
    pub(crate) async fn resolve<'a>(
        self,
        runtime: &mut deno_core::JsRuntime,
    ) -> Result<T, crate::Error> {
        let future = runtime.resolve(self.0 .0);
        let result = runtime
            .with_event_loop_future(future, Default::default())
            .await?;
        let mut scope = runtime.handle_scope();
        let local = v8::Local::new(&mut scope, &result);
        Ok(deno_core::serde_v8::from_v8(&mut scope, local)?)
    }

    /// Returns a future that resolves the promise
    pub async fn into_future<'a>(self, runtime: &mut crate::Runtime) -> Result<T, crate::Error> {
        self.resolve(runtime.deno_runtime()).await
    }

    /// Blocks until the promise is resolved
    pub fn into_value(self, runtime: &mut crate::Runtime) -> Result<T, crate::Error> {
        runtime.run_async_task(move |runtime| async move { self.into_future(runtime).await })
    }

    /// Returns the underlying v8 value
    pub fn into_v8(self) -> v8::Global<v8::Value> {
        self.0 .0
    }
}
impl<'de, T> serde::Deserialize<'de> for Promise<T>
where
    T: serde::de::DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = V8Value::<PromiseTypeChecker>::deserialize(deserializer)?;
        Ok(Self(inner, std::marker::PhantomData))
    }
}

/// A Deserializable javascript value, that can be stored and used later
/// Must live as long as the runtime it was birthed from
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Value(V8Value<DefaultTypeChecker>);
impl<'de> serde::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = V8Value::<DefaultTypeChecker>::deserialize(deserializer)?;
        Ok(Self(inner))
    }
}
impl Value {
    /// Converts the value to a rust type
    /// Mimics the auto-decoding using from_v8 that normally happens
    /// Note: This will not await the event loop, or resolve promises
    /// Use [js_value::Promise] for that
    pub fn into_type<T>(self, runtime: &mut crate::Runtime) -> Result<T, crate::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut scope = runtime.deno_runtime().handle_scope();
        let local = self.0.into_local(&mut scope).unwrap();
        Ok(deno_core::serde_v8::from_v8(&mut scope, local)?)
    }

    /// Returns the underlying v8 value
    pub fn into_v8(self) -> v8::Global<v8::Value> {
        self.0 .0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{json_args, Module, Runtime};

    #[test]
    fn test_function() {
        let module = Module::new(
            "test.js",
            "
            export const f = () => 42;
            export const f2 = async () => 42;
        ",
        );

        let mut runtime = Runtime::new(Default::default()).unwrap();
        let handle = runtime.load_module(&module).unwrap();

        let f: Function = runtime.get_value(Some(&handle), "f").unwrap();
        let value: usize = f.call(&mut runtime, Some(&handle), &json_args!()).unwrap();
        assert_eq!(value, 42);

        let f2: Function = runtime.get_value(Some(&handle), "f2").unwrap();
        let value: Promise<usize> = f2.call(&mut runtime, Some(&handle), &json_args!()).unwrap();
        let value = value.into_value(&mut runtime).unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_promise() {
        let module = Module::new(
            "test.js",
            "
            export const f = () => new Promise((resolve) => resolve(42));
        ",
        );

        let mut runtime = Runtime::new(Default::default()).unwrap();
        let handle = runtime.load_module(&module).unwrap();

        let f: Function = runtime.get_value(Some(&handle), "f").unwrap();
        let value: Promise<usize> = f.call(&mut runtime, Some(&handle), &json_args!()).unwrap();
        let value = value.into_value(&mut runtime).unwrap();
        assert_eq!(value, 42);
    }
}
