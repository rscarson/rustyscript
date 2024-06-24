use crate::Error;
use deno_core::{
    v8::{self, HandleScope},
    FromV8,
};
use serde::Deserialize;

/// A Serializable javascript function, that can be stored and called later
/// Must live as long as the runtime it was birthed from
///
/// WARNING: [JsFunction::stabilize] must be called -immediately- after this JsFunction is created
/// and before any other operations are performed on the source Runtime
///
/// Due to the potential for the underlying v8::Function to be garbage collected,
/// It must be converted to a global handle as soon as possible
///
/// When using [Runtime::get_function], this is done automatically
/// However, if you are returning a JsFunction from anywhere else,
/// You must call this function immediately after creation
#[derive(Eq, Hash, PartialEq, Debug)]
pub enum JsFunction<'rt> {
    /// A function that has not been stabilized yet
    /// It may be garbage collected if not stabilized
    Unstable(v8::Local<'rt, v8::Function>),

    /// A function that has been stabilized
    Stable(v8::Global<v8::Function>),
}
impl JsFunction<'_> {
    pub(crate) fn new(v: v8::Global<v8::Function>) -> Self {
        Self::Stable(v)
    }

    /// Due to the potential for the underlying v8::Function to be garbage collected,
    /// It must be converted to a global handle as soon as possible
    ///
    /// When using [Runtime::get_function], this is done automatically
    /// However, if you are returning a JsFunction from anywhere else,
    /// You must call this function immediately after creation
    pub fn stabilize(&mut self, runtime: &mut crate::Runtime) {
        self._stabilize(&mut runtime.deno_runtime().handle_scope())
    }

    /// Due to the potential for the underlying v8::Function to be garbage collected,
    /// It must be converted to a global handle as soon as possible
    ///
    /// When using [Runtime::get_function], this is done automatically
    /// However, if you are returning a JsFunction from anywhere else,
    /// You must call this function immediately after creation
    pub(crate) fn _stabilize(&mut self, scope: &mut HandleScope) {
        match self {
            Self::Unstable(f) => *self = Self::new(v8::Global::new(scope, *f)),
            Self::Stable(_) => {}
        }
    }
}

impl FromV8<'_> for JsFunction<'_> {
    type Error = crate::Error;

    fn from_v8(
        scope: &mut v8::HandleScope<'_>,
        value: v8::Local<'_, v8::Value>,
    ) -> Result<Self, Self::Error> {
        let function: v8::Local<v8::Function> = value
            .try_into()
            .or(Err(Error::Runtime("value was not a function".to_string())))?;
        let function = v8::Global::new(scope, function);
        Ok(Self::new(function))
    }
}

impl<'de> Deserialize<'de> for JsFunction<'_> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = crate::v8_serializer::Value::deserialize(deserializer)?;
        let local = value.v8_value;
        let function: v8::Local<v8::Function> = local
            .try_into()
            .or(Err(serde::de::Error::custom("value was not a function")))?;
        Ok(Self::Unstable(function))
    }
}
