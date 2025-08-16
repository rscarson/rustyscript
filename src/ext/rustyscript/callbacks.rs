#![allow(dead_code)]

use std::{future::Future, pin::Pin, rc::Rc};

use crate::Error;
use deno_core::{op2, serde_json, v8, OpState};
use paste::paste;

pub trait RsStoredCallback: 'static {
    fn call(
        &self,
        args: deno_core::serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<deno_core::serde_json::Value, Error>>>>;

    fn encode_args(
        &self,
        args: v8::Global<v8::Value>,
        scope: &mut v8::HandleScope<'_>,
    ) -> Result<deno_core::serde_json::Value, Error>;
}

pub trait RsCallback: 'static {
    /// A tuple of the arguments that the function takes
    type Arguments: serde::ser::Serialize + serde::de::DeserializeOwned;

    /// The return type of the function
    type Return: serde::ser::Serialize + 'static;

    /// Function body
    async fn body(args: Self::Arguments) -> Result<Self::Return, Error>;

    /// Convert a series of `v8::Value` objects into a tuple of arguments
    fn args_from_v8(
        args: Vec<v8::Global<v8::Value>>,
        scope: &mut v8::HandleScope,
    ) -> Result<Self::Arguments, Error>;

    fn slow_args_from_v8(
        args: Vec<v8::Global<v8::Value>>,
        scope: &mut v8::HandleScope,
    ) -> Result<deno_core::serde_json::Value, Error> {
        let args = Self::args_from_v8(args, scope)?;
        deno_core::serde_json::to_value(args).map_err(Error::from)
    }

    /// Convert a series of `v8::Value` objects into a tuple of arguments
    fn decode_v8(
        args: v8::Global<v8::Value>,
        scope: &mut v8::HandleScope,
    ) -> Result<Self::Arguments, Error> {
        let args = v8::Local::new(scope, args);
        let args = if args.is_array() {
            let args: v8::Local<v8::Array> = v8::Local::new(scope, args).try_into()?;
            let len = args.length() as usize;
            let mut result = Vec::with_capacity(len);
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
                result.push(v8::Global::new(scope, arg));
            }
            result
        } else {
            vec![v8::Global::new(scope, args)]
        };

        Self::args_from_v8(args, scope)
    }

    /// Call the function
    async fn call(
        args: v8::Global<v8::Value>,
        scope: &mut v8::HandleScope<'_>,
    ) -> Result<Self::Return, Error> {
        let args = Self::decode_v8(args, scope)?;
        Self::body(args).await
    }
}

macro_rules! codegen_function {
    ($(#[doc = $doc:literal])* fn $name:ident ($($n:ident:$t:ty),+ $(,)?) -> $r:ty $body:block ) => {
        paste! {
            #[allow(non_camel_case_types)]
            $(#[doc = $doc])*
            struct [< rscallback_ $name >]();
            impl RsCallback for [< rscallback_ $name >] {
                type Arguments = ($($t,)+);
                type Return = $r;

                fn args_from_v8(
                    args: Vec<v8::Global<v8::Value>>,
                    scope: &mut v8::HandleScope,
                ) -> Result<Self::Arguments, $crate::Error> {
                    let mut args = args.into_iter();
                    $(
                        let next = args.next().ok_or($crate::Error::Runtime(format!("Missing argument {} for {}", stringify!($n), stringify!($name))))?;
                        let next = $crate::deno_core::v8::Local::new(scope, next);
                        let $n:$t = $crate::deno_core::serde_v8::from_v8(scope, next)?;
                    )+
                    Ok(($($n,)+))
                }

                async fn body(($($n,)+): Self::Arguments) -> Result<Self::Return, $crate::Error> {
                    $body
                }
            }
            impl RsStoredCallback for [< rscallback_ $name >] {
                fn call(&self, args: $crate::deno_core::serde_json::Value)
                    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<$crate::deno_core::serde_json::Value, $crate::Error>>>> {
                    Box::pin(async move {
                        let args: <Self as RsCallback>::Arguments = $crate::deno_core::serde_json::from_value(args).map_err($crate::Error::from)?;

                        let v = Self::body(args).await?;
                        $crate::deno_core::serde_json::to_value(v).map_err($crate::Error::from)
                    })
                }

                fn encode_args(&self, args: $crate::deno_core::v8::Global<$crate::deno_core::v8::Value>, scope: &mut $crate::deno_core::v8::HandleScope<'_>) -> Result<$crate::deno_core::serde_json::Value, $crate::Error> {
                    let args = Self::decode_v8(args, scope)?;
                    Ok($crate::serde_json::to_value(args)?)
                }
            }
        }
    }
}

macro_rules! rs_fn {
    ($($(#[doc = $doc:literal])* fn $name:ident ($($n:ident:$t:ty),+ $(,)?) -> $r:ty $body:block )+) => {
        $(codegen_function! { fn $name ($($n:$t),+) -> $r $body })+
    }
}

rs_fn! {
    /// test
    fn add(a: i64, b: i64) -> i64 {
        Ok(a + b)
    }
}

#[op2(async)]
#[serde]
pub async fn run_rscallback<T: RsCallback>(
    #[serde] args: T::Arguments,
) -> Result<T::Return, Error> {
    T::body(args).await
}

type CallbackTable = std::collections::HashMap<String, Rc<Box<dyn RsStoredCallback>>>;

#[op2(async)]
#[serde]
pub fn rscallback(
    #[string] name: &str,
    #[serde] args: deno_core::serde_json::Value,
    state: &mut OpState,
) -> impl std::future::Future<Output = Result<serde_json::Value, Error>> {
    let callback = state
        .try_borrow::<CallbackTable>()
        .and_then(|t| t.get(name).cloned())
        .ok_or_else(|| Error::ValueNotCallable(name.to_string()));
    async move { callback?.call(args).await }
}
