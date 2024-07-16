use crate::Error;
use deno_core::v8::{self, HandleScope};
use std::{future::Future, pin::Pin};

/// Macro to generate async functions
macro_rules! codegen_function2 {
    (|$($n:ident:$t:ty),+| $body:block) => {
        (|| -> Box<dyn RsFunction> {
            Box::new(|scope: &mut $crate::deno_core::v8::HandleScope, args: Vec<$crate::deno_core::v8::Global<$crate::deno_core::v8::Value>>| Box::pin(async move {
                let mut args = args.into_iter();
                $(
                    // Convert `Value` to `$t`
                    let $n = args.next()
                        .ok_or($crate::error::Error::Runtime(format!("Wrong number of arguments")))?;
                    let $n = $crate::deno_core::v8::Local::new(scope, $n);
                    let $n:$t = $crate::deno_core::serde_v8::from_v8(scope, $n)?;
                )+

                // Execute the function
                let v = $body;

                // Convert the result to a `v8::Global`
                let v = $crate::deno_core::serde_v8::to_v8(scope, v)?;
                let v = $crate::deno_core::v8::Global::new(scope, v);
                Ok::<_, $crate::error::Error>(v)
            }))
        })()
    };
}

pub type RsPromise<'a> = Pin<Box<dyn Future<Output = Result<v8::Global<v8::Value>, Error>> + 'a>>;
pub trait RsFunction:
    'static + for<'a> Fn(&'a mut HandleScope, Vec<v8::Global<v8::Value>>) -> RsPromise<'a>
{
}
impl<F> RsFunction for F where
    F: 'static + for<'a> Fn(&'a mut HandleScope, Vec<v8::Global<v8::Value>>) -> RsPromise<'a>
{
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_async() {
        let mut runtime = crate::Runtime::new(Default::default()).unwrap();
        let f = codegen_function2!(|filename: String| { std::future::ready(filename).await });
        runtime.register_function2("test", f);

        runtime.eval("rustyscript.functions2.test('foo')").unwrap();
    }
}
