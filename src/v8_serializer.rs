use deno_core::v8;

/// A Serializable javascript value, that can be stored and called later
/// based on [deno_core::serde_v8::magic::transl8]
#[derive(Eq, Hash, PartialEq, Debug)]
pub struct Value<'s> {
    pub v8_value: v8::Local<'s, v8::Value>,
}
impl_magic!(Value<'_>);

impl<'s, T> From<v8::Local<'s, T>> for Value<'s>
where
    v8::Local<'s, T>: Into<v8::Local<'s, v8::Value>>,
{
    fn from(v: v8::Local<'s, T>) -> Self {
        Self { v8_value: v.into() }
    }
}

impl<'s> From<Value<'s>> for v8::Local<'s, v8::Value> {
    fn from(value: Value<'s>) -> Self {
        value.v8_value
    }
}

/// A Serializable javascript value, that can be stored and called later
/// based on [deno_core::serde_v8::magic::transl8]
#[derive(Eq, Hash, PartialEq, Debug)]
pub struct GlobalValue {
    pub v8_value: v8::Global<v8::Value>,
}
impl_magic!(GlobalValue);

impl<T> From<v8::Global<T>> for GlobalValue
where
    v8::Global<T>: Into<v8::Global<v8::Value>>,
{
    fn from(v: v8::Global<T>) -> Self {
        Self { v8_value: v.into() }
    }
}

impl From<GlobalValue> for v8::Global<v8::Value> {
    fn from(value: GlobalValue) -> Self {
        value.v8_value
    }
}
