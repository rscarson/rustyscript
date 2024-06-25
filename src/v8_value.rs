use deno_core::v8::{self, HandleScope};

/// A Serializable javascript object, that can be stored and called later
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
pub enum V8Value<'a, T>
where
    v8::Local<'a, T>: TryFrom<v8::Local<'a, v8::Value>>,
{
    /// A function that has not been stabilized yet
    /// It may be garbage collected if not stabilized
    Local(v8::Local<'a, T>),

    /// A function that has been stabilized
    Global(v8::Global<T>),
}

impl<'a, T> V8Value<'a, T>
where
    v8::Local<'a, T>: TryFrom<v8::Local<'a, v8::Value>>,
{
    pub(crate) fn from_global(global: v8::Global<T>) -> Self {
        Self::Global(global)
    }

    /// Due to the potential for the underlying v8::Function to be garbage collected,
    /// It must be converted to a global handle as soon as possible
    ///
    /// When using [Runtime::get_function], this is done automatically
    /// However, if you are returning a JsFunction from anywhere else,
    /// You must call this function immediately after creation
    pub(crate) fn into_global(&mut self, scope: &mut HandleScope) {
        match self {
            Self::Local(v) => *self = Self::Global(v8::Global::new(scope, *v)),
            Self::Global(_) => {}
        }
    }

    /// Due to the potential for the underlying v8::Function to be garbage collected,
    /// It must be converted to a global handle as soon as possible
    ///
    /// When using [Runtime::get_function], this is done automatically
    /// However, if you are returning a JsFunction from anywhere else,
    /// You must call this function immediately after creation
    pub fn stabilize(&mut self, runtime: &mut crate::Runtime) {
        self.into_global(&mut runtime.deno_runtime().handle_scope());
    }
}

impl<'de, 'a, T> serde::Deserialize<'de> for V8Value<'a, T>
where
    v8::Local<'a, T>: TryFrom<v8::Local<'a, v8::Value>>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = crate::v8_serializer::Value::deserialize(deserializer)?;
        let local = value.v8_value;
        let local: v8::Local<T> = local
            .try_into()
            .or(Err(serde::de::Error::custom("Unexpected type")))?;
        Ok(Self::Local(local))
    }
}

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
pub type JsFunction<'a> = V8Value<'a, v8::Function>;
