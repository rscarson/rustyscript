use crate::traits::ToModuleSpecifier;
use crate::{Error, Module, ModuleWrapper, Runtime, RuntimeOptions};
use deno_core::ModuleSpecifier;
use std::path::Path;

/// Evaluate a piece of non-ECMAScript-module JavaScript code
///
/// Effects on the global scope will not persist  
/// For a persistant variant, see [`Runtime::eval`]
///
/// # Arguments
/// * `javascript` - A single javascript expression
///
/// # Returns
/// A `Result` containing the deserialized result of the expression if successful,
/// or an error if execution fails, or the result cannot be deserialized.
///
/// # Errors
/// Will return an error if the runtime cannot be started (usually due to extension issues)  
/// Or if the expression is invalid, or if the result cannot be deserialized into the given type
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
    let mut runtime = Runtime::new(RuntimeOptions::default())?;
    runtime.eval(javascript)
}

/// Validates the syntax of some JS
///
/// # Arguments
/// * `javascript` - A snippet of JS code
///
/// # Returns
/// A `Result` containing a boolean determining the validity of the JS
///
/// # Errors
/// Will return an error if the runtime cannot be started (usually due to extension issues)  
/// Or if something went wrong and the validity could not be determined
///
/// # Example
///
/// ```rust
/// assert!(rustyscript::validate("5 + 5").expect("Something went wrong!"));
/// ```
pub fn validate(javascript: &str) -> Result<bool, Error> {
    let module = Module::new("test.js", javascript);
    let mut runtime = Runtime::new(RuntimeOptions::default())?;
    match runtime.load_modules(&module, vec![]) {
        Ok(_) => Ok(true),
        Err(Error::Runtime(_) | Error::JsError(_)) => Ok(false),
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
/// # Errors
/// Will return an error if the file cannot be found, execution fails, or the runtime
/// cannot be started (usually due to extension issues)
///
/// # Example
///
/// ```no_run
/// let mut module = rustyscript::import("js/my_module.js").expect("Something went wrong!");
/// ```
pub fn import(path: &str) -> Result<ModuleWrapper, Error> {
    ModuleWrapper::new_from_file(path, RuntimeOptions::default())
}

/// Resolve a path to absolute path, relative to the current working directory
/// or an optional base directory
///
/// The resulting `ModuleSpecifier` is a wrapper around `reqwest::Url`
///
/// # Arguments
/// * `path` - A path
/// * `base_dir` - An optional base directory to resolve the path from  
///                If not provided, the current working directory is used
///
/// # Errors
/// Will return an error if the given path is invalid
///
/// # Example
///
/// ```rust
/// rustyscript::resolve_path("test.js", None).expect("Something went wrong!");
/// ```
pub fn resolve_path(
    path: impl AsRef<std::path::Path>,
    base_dir: Option<&Path>,
) -> Result<ModuleSpecifier, Error> {
    let path = path.as_ref();
    let url = match base_dir {
        Some(dir) => path.to_module_specifier(dir),
        None => path.to_module_specifier(&std::env::current_dir()?),
    }?;

    Ok(url)
}

/// Explicitly initialize the V8 platform  
/// Note that all runtimes must have a common parent thread that initalized the V8 platform
///
/// This is done automatically the first time [`Runtime::new`] is called,
/// but for multi-threaded applications, it may be necessary to call this function manually
pub fn init_platform(thread_pool_size: u32, idle_task_support: bool) {
    let platform = deno_core::v8::Platform::new(thread_pool_size, idle_task_support);
    deno_core::JsRuntime::init_platform(Some(platform.into()), true);
}

#[macro_use]
mod runtime_macros {
    /// Map a series of values into a form which javascript functions can understand
    ///
    /// Accepts a maximum of 16 arguments, of any combination of compatible types  
    /// For more than 16 arguments, use `big_json_args!` instead
    ///
    /// NOTE: Since 0.6.0, this macro is now effectively a no-op  
    /// It simply builds a tuple reference from the provided arguments
    ///
    /// You can also just pass a &tuple directly, or an &array, or even a single value
    ///
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
        ($($arg:expr),*) => {
            &($($arg),*)
        };
    }

    /// Map a series of values into a form which javascript functions can understand  
    /// This forms a `Vec<serde_json::Value>` from the provided arguments
    ///
    /// Useful if you need more than 16 arguments for a single function call
    ///
    /// Warning: This macro is far slower than `json_args!` and should be used sparingly  
    /// Benchmarks place the performance difference at nearly 1,000 times slower!
    ///
    /// # Example
    /// ```rust
    /// use rustyscript::{ Runtime, RuntimeOptions, Module, big_json_args };
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
    ///     big_json_args!("test", 5)
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    #[macro_export]
    macro_rules! big_json_args {
        ($($arg:expr),*) => {
            &vec![
                $($crate::deno_core::serde_json::Value::from($arg)),*
            ]
        };
    }

    /// A simple helper macro to create a callback for use with `Runtime::register_function`  
    /// Takes care of deserializing arguments and serializing the result
    ///
    /// # Example
    /// ```rust
    /// use rustyscript::{ Error, sync_callback };
    /// let add = sync_callback!(
    ///     |a: i64, b: i64| {
    ///         Ok::<i64, Error>(a + b)
    ///     }
    /// );
    /// ```
    #[macro_export]
    macro_rules! sync_callback {
        (|$($arg:ident: $arg_ty:ty),*| $body:expr) => {
            |args: &[$crate::serde_json::Value]| {
                let mut args = args.iter();
                $(
                    let $arg: $arg_ty = match args.next() {
                        Some(arg) => $crate::serde_json::from_value(arg.clone())?,
                        None => return Err($crate::Error::Runtime("Invalid number of arguments".to_string())),
                    };
                )*
                let result = $body?;
                $crate::serde_json::Value::try_from(result).map_err(|e| $crate::Error::Runtime(e.to_string()))
            }
        }
    }

    /// A simple helper macro to create a callback for use with `Runtime::register_async_function`  
    /// Takes care of deserializing arguments and serializing the result
    ///
    /// # Example
    /// ```rust
    /// use rustyscript::{ Error, async_callback };
    /// let add = async_callback!(
    ///     |a: i64, b: i64| async move {
    ///         Ok::<i64, Error>(a + b)
    ///     }
    /// );
    /// ```
    #[macro_export]
    macro_rules! async_callback {
        (|$($arg:ident: $arg_ty:ty),*| $body:expr) => {
            |args: Vec<$crate::serde_json::Value>| Box::pin(async move {
                let mut args = args.iter();
                $(
                    let $arg: $arg_ty = match args.next() {
                        Some(arg) => $crate::serde_json::from_value(arg.clone()).map_err(|e| $crate::Error::Runtime(e.to_string()))?,
                        None => return Err($crate::Error::Runtime("Invalid number of arguments".to_string())),
                    };
                )*

                // Now consume the future to inject JSON serialization
                let result = $body.await?;
                $crate::serde_json::Value::try_from(result).map_err(|e| $crate::Error::Runtime(e.to_string()))
            })
        }
    }
}

#[cfg(test)]
mod test_runtime {
    use super::*;
    use deno_core::{futures::FutureExt, serde_json};

    #[test]
    fn test_callback() {
        let add = sync_callback!(|a: i64, b: i64| { Ok::<i64, Error>(a + b) });

        let add2 = async_callback!(|a: i64, b: i64| async move { Ok::<i64, Error>(a + b) });

        let args = vec![
            serde_json::Value::Number(5.into()),
            serde_json::Value::Number(5.into()),
        ];
        let result = add(&args).unwrap();
        assert_eq!(serde_json::Value::Number(10.into()), result);

        let result = add2(args).now_or_never().unwrap().unwrap();
        assert_eq!(serde_json::Value::Number(10.into()), result);
    }

    #[test]
    fn test_evaluate() {
        assert_eq!(5, evaluate::<i64>("3 + 2").expect("invalid expression"));
        evaluate::<i64>("a5; 3 + 2").expect_err("Expected an error");
    }

    #[test]
    fn test_validate() {
        assert!(validate("3 + 2").expect("invalid expression"));
        assert!(!validate("5;+-").expect("invalid expression"));
    }

    #[test]
    fn test_resolve_path() {
        assert!(resolve_path("test.js", None)
            .expect("invalid path")
            .to_string()
            .ends_with("test.js"));
    }
}
