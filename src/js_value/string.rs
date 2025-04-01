use super::V8Value;
use deno_core::v8::{self, HandleScope, WriteFlags};
use serde::Deserialize;

/// A Deserializable javascript UTF-16 string, that can be stored and used later
/// Must live as long as the runtime it was birthed from
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct String(V8Value<StringTypeChecker>);
impl_v8!(String, StringTypeChecker);
impl_checker!(StringTypeChecker, String, is_string, |e| {
    crate::Error::JsonDecode(format!("Expected a string, found `{e}`"))
});

impl String {
    /// Converts the string to a rust string
    /// Potentially lossy, if the string contains orphan UTF-16 surrogates
    pub fn to_string_lossy(&self, runtime: &mut crate::Runtime) -> std::string::String {
        let mut scope = runtime.deno_runtime().handle_scope();
        self.to_rust_string_lossy(&mut scope)
    }

    /// Converts the string to a rust string
    /// If the string contains orphan UTF-16 surrogates, it may return None
    /// In that case, you can use `to_string_lossy` to get a lossy conversion
    pub fn to_string(&self, runtime: &mut crate::Runtime) -> Option<std::string::String> {
        let bytes = self.to_utf8_bytes(runtime);
        std::string::String::from_utf8(bytes).ok()
    }

    /// Converts the string to a UTF-8 character buffer in the form of a `Vec<u8>`
    /// Excludes the null terminator
    pub fn to_utf8_bytes(&self, runtime: &mut crate::Runtime) -> Vec<u8> {
        let mut scope = runtime.deno_runtime().handle_scope();
        self.to_utf8_buffer(&mut scope)
    }

    /// Converts the string to a UTF-16 character buffer in the form of a `Vec<u16>`
    /// Excludes the null terminator
    pub fn to_utf16_bytes(&self, runtime: &mut crate::Runtime) -> Vec<u16> {
        let mut scope = runtime.deno_runtime().handle_scope();
        self.to_utf16_buffer(&mut scope)
    }

    pub(crate) fn to_rust_string_lossy(&self, scope: &mut HandleScope<'_>) -> std::string::String {
        let local = self.0.as_local(scope);
        local.to_rust_string_lossy(scope)
    }

    pub(crate) fn to_utf16_buffer(&self, scope: &mut HandleScope<'_>) -> Vec<u16> {
        let local = self.0.as_local(scope);
        let u16_len = local.length();
        let mut buffer = vec![0; u16_len];

        local.write_v2(scope, 0, &mut buffer, WriteFlags::empty());
        buffer
    }

    pub(crate) fn to_utf8_buffer(&self, scope: &mut HandleScope<'_>) -> Vec<u8> {
        let local = self.0.as_local(scope);
        let u8_len = local.utf8_length(scope);
        let mut buffer = vec![0; u8_len];

        local.write_utf8_v2(scope, &mut buffer, WriteFlags::empty(), None);
        buffer
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Module, Runtime, RuntimeOptions};

    #[test]
    fn test_string() {
        let module = Module::new(
            "test.js",
            "
            // Valid UTF-8
            export const good = 'Hello, World!';

            // Invalid UTF-8, valid UTF-16
            export const bad = '\\ud83d\\ude00';
        ",
        );

        let mut runtime = Runtime::new(RuntimeOptions::default()).unwrap();
        let handle = runtime.load_module(&module).unwrap();

        let f: String = runtime.get_value(Some(&handle), "good").unwrap();
        let value = f.to_string_lossy(&mut runtime);
        assert_eq!(value, "Hello, World!");

        let f: String = runtime.get_value(Some(&handle), "good").unwrap();
        let value = f.to_string(&mut runtime).unwrap();
        assert_eq!(value, "Hello, World!");

        let f: String = runtime.get_value(Some(&handle), "bad").unwrap();
        let value = f.to_utf16_bytes(&mut runtime);
        assert_eq!(value, vec![0xd83d, 0xde00]);
    }
}
