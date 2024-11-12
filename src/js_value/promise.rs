use crate::async_bridge::AsyncBridgeExt;

use super::V8Value;
use deno_core::{
    v8::{self},
    PollEventLoopOptions,
};
use serde::Deserialize;

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
impl_checker!(PromiseTypeChecker, Promise, is_promise, |e| {
    crate::Error::JsonDecode(format!("Expected a promise, found `{e}`"))
});

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
            .with_event_loop_future(future, PollEventLoopOptions::default())
            .await?;
        let mut scope = runtime.handle_scope();
        let local = v8::Local::new(&mut scope, &result);
        Ok(deno_core::serde_v8::from_v8(&mut scope, local)?)
    }

    /// Returns a future that resolves the promise
    ///
    /// # Errors
    /// Will return an error if the promise cannot be resolved into the given type,
    /// or if a runtime error occurs
    pub async fn into_future<'a>(self, runtime: &mut crate::Runtime) -> Result<T, crate::Error> {
        self.resolve(runtime.deno_runtime()).await
    }

    /// Blocks until the promise is resolved
    ///
    /// # Errors
    /// Will return an error if the promise cannot be resolved into the given type,
    /// or if a runtime error occurs
    pub fn into_value(self, runtime: &mut crate::Runtime) -> Result<T, crate::Error> {
        runtime.block_on(move |runtime| async move { self.into_future(runtime).await })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{js_value::Function, json_args, Module, Runtime, RuntimeOptions};

    #[test]
    fn test_promise() {
        let module = Module::new(
            "test.js",
            "
            export const f = () => new Promise((resolve) => resolve(42));
        ",
        );

        let mut runtime = Runtime::new(RuntimeOptions::default()).unwrap();
        let handle = runtime.load_module(&module).unwrap();

        let f: Function = runtime.get_value(Some(&handle), "f").unwrap();
        let value: Promise<usize> = f
            .call_immediate(&mut runtime, Some(&handle), &json_args!())
            .unwrap();
        let value = value.into_value(&mut runtime).unwrap();
        assert_eq!(value, 42);
    }
}
