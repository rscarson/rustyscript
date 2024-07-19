use crate::Error;
use deno_core::resolve_path;
use deno_core::v8::{self, HandleScope};
use deno_core::ModuleSpecifier;
use std::env::current_dir;
use std::path::Path;

pub trait ToModuleSpecifier {
    fn to_module_specifier(&self, base: Option<&Path>) -> Result<ModuleSpecifier, Error>;
}

impl ToModuleSpecifier for str {
    fn to_module_specifier(&self, base: Option<&Path>) -> Result<ModuleSpecifier, Error> {
        let base = match base {
            Some(base) => base,
            None => &current_dir()?,
        };
        resolve_path(self, base).map_err(Error::from)
    }
}

pub trait ToV8String {
    fn to_v8_string<'a>(
        &self,
        scope: &mut HandleScope<'a>,
    ) -> Result<v8::Local<'a, v8::String>, Error>;
}

impl ToV8String for str {
    fn to_v8_string<'a>(
        &self,
        scope: &mut HandleScope<'a>,
    ) -> Result<v8::Local<'a, v8::String>, Error> {
        v8::String::new(scope, self).ok_or(Error::V8Encoding(self.to_string()))
    }
}

pub trait ToDefinedValue<T> {
    fn if_defined(&self) -> Option<T>;
}

impl<'a> ToDefinedValue<v8::Local<'a, v8::Value>> for Option<v8::Local<'a, v8::Value>> {
    fn if_defined(&self) -> Option<v8::Local<'a, v8::Value>> {
        self.filter(|v| !v.is_undefined())
    }
}
