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

        #[allow(clippy::from_over_into)]
        impl $(<$generic>)? Into<v8::Global<v8::Value>> for $name $(<$generic>)? $(where $generic: serde::de::DeserializeOwned)? {
            fn into(self) -> v8::Global<v8::Value> {
                self.0 .0
            }
        }
    };
}

/// A macro to implement type checkers for [Function], [Promise], and [Value]
macro_rules! impl_checker {
    ($name:ident, $v8_name:ident, $checker_fn:ident, |$err_ty:ident| $err:block) => {
        #[doc = "Implementations of `V8TypeChecker`"]
        #[doc = concat!("Guards for `v8::", stringify!($v8_name), "` values")]
        #[derive(Eq, Hash, PartialEq, Debug, Clone, Deserialize)]
        struct $name;
        impl $crate::js_value::V8TypeChecker for $name {
            type Output = v8::$v8_name;
            fn validate(value: v8::Global<v8::Value>) -> Result<(), crate::Error> {
                let raw: &v8::Value = unsafe { v8::Handle::get_unchecked(&value) };
                if raw.$checker_fn() {
                    Ok(())
                } else {
                    let $err_ty = raw.type_repr().to_string();
                    Err($err)
                }
            }
        }
    };

    ($name:ident, $v8_name:ident) => {
        #[doc = "Implementation of `V8TypeChecker`"]
        #[doc = concat!("Guards for `v8::", stringify!($v8_name), "` values")]
        #[derive(Eq, Hash, PartialEq, Debug, Clone, Deserialize)]
        struct $name;
        impl V8TypeChecker for $name {
            type Output = v8::$v8_name;
            fn validate(_: v8::Global<v8::Value>) -> Result<(), crate::Error> {
                Ok(())
            }
        }
    };
}

/// A trait that is used to check if a `v8::Value` is of a certain type
/// Will cause a panic if validate is insufficient to verify that the
/// given value is of type `T::Output`
pub(crate) trait V8TypeChecker {
    /// The v8 type that this checker guards for
    type Output;

    /// Checks if a `v8::Value` is of the output type
    /// If the value is not of the output type, an error is returned
    ///
    /// Note: If the guard is not sufficient to verify the type, a panic will occur
    /// when this checker is used
    fn validate(value: v8::Global<v8::Value>) -> Result<(), crate::Error>;
}

// For values
impl_checker!(DefaultTypeChecker, Value);

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

    /// Contructs a new Value from a v8::Value global
    pub fn from_v8(value: v8::Global<v8::Value>) -> Result<Self, crate::Error> {
        DefaultTypeChecker::validate(value.clone())?;
        Ok(Self(V8Value(value, std::marker::PhantomData)))
    }
}

mod function;
pub use function::*;

mod promise;
pub use promise::*;

mod string;
pub use string::*;

mod map;
pub use map::*;

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Module, Runtime};

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
}
