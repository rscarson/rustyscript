use std::{borrow::Cow, path::Path};

use deno_core::{
    v8::{self, HandleScope},
    ModuleSpecifier,
};

use crate::Error;

/// Converts a string representing a relative or absolute path into a
/// `ModuleSpecifier`. A relative path is considered relative to the passed
/// `current_dir`.
///
/// This is a patch for the str only `deno_core` provided version
fn resolve_path(
    path_str: impl AsRef<Path>,
    current_dir: &Path,
) -> Result<ModuleSpecifier, deno_core::ModuleResolutionError> {
    let path = current_dir.join(path_str);
    let path = deno_core::normalize_path(Cow::Owned(path));
    deno_core::url::Url::from_file_path(&path).map_err(|()| {
        deno_core::ModuleResolutionError::InvalidUrl(
            deno_core::url::ParseError::RelativeUrlWithoutBase,
        )
    })
}

pub trait ToModuleSpecifier {
    fn to_module_specifier(&self, base: &Path) -> Result<ModuleSpecifier, Error>;
}

impl<T: AsRef<Path>> ToModuleSpecifier for T {
    fn to_module_specifier(&self, base: &Path) -> Result<ModuleSpecifier, Error> {
        Ok(resolve_path(self, base)?)
    }
}

/// Convert a string to a V8 string
///
/// **DEPRECATED**: This trait is deprecated and may be removed in a future version.
/// Use `v8::String::new(scope, string)` directly instead.
///
/// # Migration Guide
/// ```rust,ignore
/// // Old code:
/// use rustyscript::ToV8String;
/// let v8_str = my_string.to_v8_string(scope)?;
///
/// // New code:
/// let v8_str = v8::String::new(scope, my_string)
///     .ok_or_else(|| Error::V8Encoding(my_string.to_string()))?;
/// ```
#[deprecated(
    since = "0.8.0",
    note = "Use v8::String::new() directly. This trait will be removed in a future version."
)]
pub trait ToV8String {
    /// Convert this value to a V8 string
    ///
    /// # Errors
    /// Returns an error if the string cannot be encoded as a V8 string
    fn to_v8_string<'a>(
        &self,
        scope: &mut HandleScope<'a>,
    ) -> Result<v8::Local<'a, v8::String>, Error>;
}

#[allow(deprecated)]
impl ToV8String for str {
    fn to_v8_string<'a>(
        &self,
        scope: &mut HandleScope<'a>,
    ) -> Result<v8::Local<'a, v8::String>, Error> {
        // SAFETY: The V8 API requires &PinnedRef<HandleScope> but we have &mut HandleScope.
        // This is safe because the HandleScope is already on the stack and pinned.
        // This pattern is used throughout the codebase for V8 API compatibility.
        let scope_ref: &v8::PinnedRef<HandleScope<'a>> = unsafe {
            &*std::ptr::from_mut(scope).cast::<v8::PinnedRef<HandleScope<'a>>>()
        };
        v8::String::new(scope_ref, self).ok_or_else(|| Error::V8Encoding(self.to_string()))
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

// Note: ToV8String trait is tested implicitly through its usage in the codebase.
// Direct V8 API testing is complex and not necessary for a deprecated trait that
// already has working implementation and usage in production code.
