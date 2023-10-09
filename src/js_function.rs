use deno_core::v8::{self, HandleScope};
use serde::{Deserialize, Deserializer, Serialize};

/// A Serializable javascript function, that can be stored and called later
/// Must live as long as the runtime it was birthed from
#[derive(Eq, Hash, PartialEq)]
pub struct JsFunction<'a>(v8::Local<'a, v8::Function>);
impl<'a> JsFunction<'a> {
    /// Extract the underlying v8::Function object
    /// Use `Runtime::call_stored_function` instead!
    pub fn to_v8(&self) -> v8::Local<'_, v8::Function> {
        self.0
    }

    /// Extract the underlying v8::Function object as a global
    /// Use `Runtime::call_stored_function` instead!
    pub fn to_v8_global(&self, scope: &mut HandleScope<'a>) -> v8::Global<v8::Function> {
        v8::Global::new(scope, self.to_v8())
    }
}

impl Serialize for JsFunction<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let v: v8::Local<v8::Value> = self.0.into();
        deno_core::serde_v8::Value::from(v).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for JsFunction<'_> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = deno_core::serde_v8::Value::deserialize(deserializer)?;
        let value = value.v8_value;
        let function: v8::Local<v8::Function> = value
            .try_into()
            .or(Err(serde::de::Error::custom("value was not a function")))?;
        Ok(Self(function))
    }
}
