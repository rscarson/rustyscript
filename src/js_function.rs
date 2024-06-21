use deno_core::v8::{self, HandleScope};
use serde::{Deserialize, Deserializer, Serialize};

use crate::Runtime;

/// A Serializable javascript function, that can be stored and called later
/// Must live as long as the runtime it was birthed from
///
/// WARNING: [JsFunction::stabilize] must be called -immediately- after this JsFunction is created
/// and before any other operations are performed on the source Runtime
#[derive(Eq, Hash, PartialEq, Debug)]
pub enum JsFunction<'s> {
    /// Temporary representation created by the deserializer
    /// Works, but only if used right away
    Local(v8::Local<'s, v8::Function>),

    /// Global representation, created by [JsFunction::stabilize]
    /// Can be used at any time
    Global(v8::Global<v8::Function>),
}

impl<'s> JsFunction<'s> {
    /// Extract the underlying v8::Function object as a global
    /// Use `Runtime::call_stored_function` instead!
    pub(crate) fn to_v8_global(&self, scope: &mut HandleScope<'s>) -> v8::Global<v8::Function> {
        match self {
            Self::Local(local) => v8::Global::new(scope, *local),
            Self::Global(global) => global.clone(),
        }
    }

    pub(crate) fn to_v8_local(&self) -> Option<v8::Local<'s, v8::Function>> {
        match self {
            Self::Local(local) => Some(*local),
            Self::Global(_) => None,
        }
    }

    /// Convert this function to a global function
    /// WARNING: This must be called -immediately- after this JsFunction is created
    /// and before any other operations are performed on the source Runtime
    pub fn stabilize(&mut self, runtime: &mut Runtime) {
        self.as_global(&mut runtime.deno_runtime().handle_scope())
    }

    /// Convert this function to a global function
    /// WARNING: This must be called -immediately- after this JsFunction is created
    /// and before any other operations are performed on the source Runtime
    pub(crate) fn as_global(&mut self, scope: &mut HandleScope) {
        match self {
            Self::Local(local) => *self = Self::Global(v8::Global::new(scope, *local)),
            Self::Global(_) => (),
        }
    }
}

impl Serialize for JsFunction<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let v = self.to_v8_local().ok_or(serde::ser::Error::custom(
            "Could not serialize this function",
        ))?;
        let v: v8::Local<v8::Value> = v.into();
        crate::v8_serializer::Value::from(v).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for JsFunction<'_> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = crate::v8_serializer::Value::deserialize(deserializer)?;
        let value = value.v8_value;
        let function: v8::Local<v8::Function> = value
            .try_into()
            .or(Err(serde::de::Error::custom("value was not a function")))?;

        Ok(Self::Local(function))
    }
}
