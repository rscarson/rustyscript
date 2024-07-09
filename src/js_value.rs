//! This module provides a way to store and use javascript values, functions, and promises
//! The are a deserialized version of the v8::Value
//!
//! [Function] and [Promise] are both specializations of [Value] providing deserialize-time type checking
//! and additional utility functions for interacting with the runtime
use deno_core::serde_v8::GlobalValue;
use deno_core::v8::{self, HandleScope};
use serde::Deserialize;

/// A macro to implement the common functions for [Function], [Promise], and [Value]
macro_rules! impl_v8 {
    ($name:ident$(<$generic:ident>)?, $checker:ident $(,)?) => {
        impl $(<$generic>)? $name $(<$generic>)? where
        $( $generic: serde::de::DeserializeOwned, )? {
            /// Returns the underlying [deno_core::v8::Global]
            /// This is useful if you want to pass the value to a [deno_core::JsRuntime] function directly
            pub fn into_v8(self) -> v8::Global<v8::Value> {
                self.0 .0
            }
        }
        impl<'de$(, $generic)?> serde::Deserialize<'de> for $name $(<$generic>)?
        $(where $generic: serde::de::DeserializeOwned,)?
        {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let inner = V8Value::<$checker>::deserialize(deserializer)?;
                Ok(Self(inner $(, std::marker::PhantomData::<$generic>)?))
            }
        }
        impl $(<$generic>)? Into<v8::Global<v8::Value>> for $name $(<$generic>)? $(where $generic: serde::de::DeserializeOwned)? {
            fn into(self) -> v8::Global<v8::Value> {
                self.0 .0
            }
        }
    };
}

/// A trait that is used to check if a `v8::Value` is of a certain type
/// Will cause a panic if validate is insufficient to verify that the
/// given value is of type `T::Output`
trait V8TypeChecker {
    type Output;

    /// Converts a `v8::Global<v8::Value>` to a `&v8::Value`
    ///
    /// # Safety
    /// non-existant. Do not call this function
    fn into_raw(value: &v8::Global<v8::Value>) -> &v8::Value {
        unsafe { v8::Handle::get_unchecked(value) }
    }

    /// Checks if a `v8::Value` is of a certain type
    fn validate(value: v8::Global<v8::Value>) -> Result<(), crate::Error>;
}

/// Implementations of `V8TypeChecker` for functions
#[derive(Eq, Hash, PartialEq, Debug, Clone, Deserialize)]
struct FunctionTypeChecker;
impl V8TypeChecker for FunctionTypeChecker {
    type Output = v8::Function;
    fn validate(value: v8::Global<v8::Value>) -> Result<(), crate::Error> {
        let raw = Self::into_raw(&value);
        if raw.is_function() {
            Ok(())
        } else {
            Err(crate::Error::ValueNotCallable(raw.type_repr().to_string()))
        }
    }
}

/// Implementations of `V8TypeChecker` for promises
#[derive(Eq, Hash, PartialEq, Debug, Clone, Deserialize)]
struct PromiseTypeChecker;
impl V8TypeChecker for PromiseTypeChecker {
    type Output = v8::Promise;
    fn validate(value: v8::Global<v8::Value>) -> Result<(), crate::Error> {
        let raw = Self::into_raw(&value);
        if raw.is_promise() {
            Ok(())
        } else {
            Err(crate::Error::JsonDecode(format!(
                "Expected a promise, found `{}`",
                raw.type_repr()
            )))
        }
    }
}

/// Implementations of `V8TypeChecker` for any value
#[derive(Eq, Hash, PartialEq, Debug, Clone, Deserialize)]
struct DefaultTypeChecker;
impl V8TypeChecker for DefaultTypeChecker {
    type Output = v8::Value;
    fn validate(_value: v8::Global<v8::Value>) -> Result<(), crate::Error> {
        Ok(())
    }
}

/// The core struct behind the [Function], [Promise], and [Value] types
/// Should probably not be user-facing
/// TODO: Safer API for this so we can make it public eventually
///
/// A Deserializable javascript object, that can be stored and used later
/// Must live as long as the runtime it was birthed from
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
struct V8Value<V8TypeChecker>(
    v8::Global<v8::Value>,
    std::marker::PhantomData<V8TypeChecker>,
);

impl<T: V8TypeChecker> V8Value<T> {
    /// Returns the underlying global as a local in the type configured by the type checker
    pub(crate) fn as_local<'a>(&self, scope: &mut HandleScope<'a>) -> v8::Local<'a, T::Output>
    where
        v8::Local<'a, T::Output>: TryFrom<v8::Local<'a, v8::Value>>,
    {
        let local = v8::Local::new(scope, &self.0);
        v8::Local::<'a, T::Output>::try_from(local)
            .ok()
            .expect("Failed to convert V8Value: Invalid V8TypeChecker!")
    }

    /// Returns the underlying global in the type configured by the type checker
    pub(crate) fn as_global<'a>(&self, scope: &mut HandleScope<'a>) -> v8::Global<T::Output>
    where
        v8::Local<'a, T::Output>: TryFrom<v8::Local<'a, v8::Value>>,
    {
        let local = self.as_local(scope);
        v8::Global::new(scope, local)
    }
}

impl<'de, T: V8TypeChecker> serde::Deserialize<'de> for V8Value<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = GlobalValue::deserialize(deserializer)?;
        T::validate(value.v8_value.clone()).map_err(serde::de::Error::custom)?;
        Ok(Self(value.v8_value, std::marker::PhantomData))
    }
}

/// A Deserializable javascript function, that can be stored and used later
/// Must live as long as the runtime it was birthed from
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Function(V8Value<FunctionTypeChecker>);
impl_v8!(Function, FunctionTypeChecker);
impl Function {
    pub(crate) fn as_global(&self, scope: &mut HandleScope<'_>) -> v8::Global<v8::Function> {
        self.0.as_global(scope)
    }

    /// Returns true if the function is async
    pub fn is_async(&self) -> bool {
        // Safe because we aren't applying this to an isolate
        let unsafe_f = unsafe { v8::Handle::get_unchecked(&self.0 .0) };
        unsafe_f.is_async_function()
    }

    /// Calls this function. See [crate::Runtime::call_stored_function]
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

    /// Calls this function. See [crate::Runtime::call_stored_function_async]
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

    /// Calls this function. See [crate::Runtime::call_stored_function_immediate]
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
impl_v8!(Promise<T>, PromiseTypeChecker);
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
}

/// A Deserializable javascript value, that can be stored and used later
/// Can only be used on the same runtime it was created on
///
/// This mimics the auto-decoding that happens when providing a type parameter to Runtime functions
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Value(V8Value<DefaultTypeChecker>);
impl_v8!(Value, DefaultTypeChecker);
impl Value {
    /// Converts the value to an arbitrary rust type
    /// Mimics the auto-decoding using from_v8 that normally happens
    /// Note: This will not await the event loop, or resolve promises
    /// Use [crate::js_value::Promise] as the generic T for that
    pub fn try_into<T>(self, runtime: &mut crate::Runtime) -> Result<T, crate::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut scope = runtime.deno_runtime().handle_scope();
        let local = self.0.as_local(&mut scope);
        Ok(deno_core::serde_v8::from_v8(&mut scope, local)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{json_args, Module, Runtime};

    #[test]
    fn test_value() {
        let module = Module::new(
            "test.js",
            "
            export const f = 42;
        ",
        );

        let mut runtime = Runtime::new(Default::default()).unwrap();
        let handle = runtime.load_module(&module).unwrap();

        let f: Value = runtime.get_value(Some(&handle), "f").unwrap();
        let value: usize = f.try_into(&mut runtime).unwrap();
        assert_eq!(value, 42);
    }

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
        let value: Promise<usize> = f2
            .call_immediate(&mut runtime, Some(&handle), &json_args!())
            .unwrap();
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
        let value: Promise<usize> = f
            .call_immediate(&mut runtime, Some(&handle), &json_args!())
            .unwrap();
        let value = value.into_value(&mut runtime).unwrap();
        assert_eq!(value, 42);
    }
}
