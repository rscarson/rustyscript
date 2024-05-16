use crate::traits::ToModuleSpecifier;
use crate::{Error, Module, ModuleWrapper, Runtime};

/// Evaluate a piece of non-ECMAScript-module JavaScript code
/// Effects on the global scope will not persist
/// For a persistant variant, see [Runtime::eval]
///
/// # Arguments
/// * `javascript` - A single javascript expression
///
/// # Returns
/// A `Result` containing the deserialized result of the expression if successful,
/// or an error if execution fails, or the result cannot be deserialized.
///
/// # Example
///
/// ```rust
/// let result: i64 = rustyscript::evaluate("5 + 5").expect("The expression was invalid!");
/// assert_eq!(10, result);
/// ```
pub fn evaluate<T>(javascript: &str) -> Result<T, Error>
where
    T: deno_core::serde::de::DeserializeOwned,
{
    let mut runtime = Runtime::new(Default::default())?;
    runtime.eval(javascript)
}

/// Validates the syntax of some JS
///
/// # Arguments
/// * `javascript` - A snippet of JS code
///
/// # Returns
/// A `Result` containing a boolean determining the validity of the JS,
/// or an error if something went wrong.
///
/// # Example
///
/// ```rust
/// assert!(rustyscript::validate("5 + 5").expect("Something went wrong!"));
/// ```
pub fn validate(javascript: &str) -> Result<bool, Error> {
    let module = Module::new("test.js", javascript);
    let mut runtime = Runtime::new(Default::default())?;
    match runtime.load_modules(&module, vec![]) {
        Ok(_) => Ok(true),
        Err(Error::Runtime(_)) => Ok(false),
        Err(Error::JsError(_)) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Imports a JS module into a new runtime
///
/// # Arguments
/// * `path` - Path to the JS module to import
///
/// # Returns
/// A `Result` containing a handle to the imported module,
/// or an error if something went wrong.
///
/// # Example
///
/// ```no_run
/// let mut module = rustyscript::import("js/my_module.js").expect("Something went wrong!");
/// ```
pub fn import(path: &str) -> Result<ModuleWrapper, Error> {
    ModuleWrapper::new_from_file(path, Default::default())
}

/// Resolve a path to absolute path
///
/// # Arguments
/// * `path` - A path
///
/// # Example
///
/// ```rust
/// let full_path = rustyscript::resolve_path("test.js").expect("Something went wrong!");
/// assert!(full_path.ends_with("test.js"));
/// ```
pub fn resolve_path(path: &str) -> Result<String, Error> {
    Ok(path.to_module_specifier()?.to_string())
}

#[macro_use]
mod runtime_macros {
    /// Map a series of values to a slice of `serde_json::Value` objects
    /// that javascript functions can understand
    /// # Example
    /// ```rust
    /// use rustyscript::{ Runtime, RuntimeOptions, Module, json_args };
    /// use std::time::Duration;
    ///
    /// # fn main() -> Result<(), rustyscript::Error> {
    /// let module = Module::new("test.js", "
    ///     function load(a, b) {
    ///         console.log(`Hello world: a=${a}, b=${b}`);
    ///     }
    ///     rustyscript.register_entrypoint(load);
    /// ");
    ///
    /// Runtime::execute_module(
    ///     &module, vec![],
    ///     Default::default(),
    ///     json_args!("test", 5)
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    #[macro_export]
    macro_rules! json_args {
        ($($arg:expr),+) => {
            &[
                $($crate::Runtime::into_arg($arg)),+
            ]
        };

        () => {
            $crate::Runtime::EMPTY_ARGS
        };
    }
}

#[cfg(test)]
mod test_runtime {
    use super::*;

    #[test]
    fn test_evaluate() {
        assert_eq!(5, evaluate::<i64>("3 + 2").expect("invalid expression"));
        evaluate::<i64>("5; 3 + 2").expect_err("invalid expression");
    }

    #[test]
    fn test_validate() {
        assert_eq!(true, validate("3 + 2").expect("invalid expression"));
        assert_eq!(false, validate("5;+-").expect("invalid expression"));
    }

    #[test]
    fn test_resolve_path() {
        assert!(resolve_path("test.js")
            .expect("invalid path")
            .ends_with("test.js"));
    }
}
