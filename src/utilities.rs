use crate::traits::ToModuleSpecifier;
use crate::{Error, Runtime, Script};

/// Execute a single JS expression
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
/// let result: i64 = js_playground::evaluate("5 + 5").expect("The expression was invalid!");
/// assert_eq!(10, result);
/// ```
pub fn evaluate<T>(javascript: &str) -> Result<T, Error>
where
    T: deno_core::serde::de::DeserializeOwned,
{
    let script = Script::new(
        "js_eval.js",
        &format!(
            "
        export function js_playground_evaluate(){{
            return ({javascript});
        }}
    "
        ),
    );
    let mut runtime = Runtime::new(Default::default())?;
    let module = runtime.load_modules(&script, vec![])?;
    runtime.call_function(&module, "js_playground_evaluate", Runtime::EMPTY_ARGS)
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
/// assert!(js_playground::validate("5 + 5").expect("Something went wrong!"));
/// ```
pub fn validate(javascript: &str) -> Result<bool, Error> {
    let script = Script::new("test.js", javascript);
    let mut runtime = Runtime::new(Default::default())?;
    match runtime.load_modules(&script, vec![]) {
        Ok(_) => Ok(true),
        Err(e) if matches!(e, Error::Runtime(_)) => Ok(false),
        Err(e) => Err(e),
    }
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
/// let full_path = js_playground::resolve_path("test.js").expect("Something went wrong!");
/// assert!(full_path.ends_with("test.js"));
/// ```
pub fn resolve_path(path: &str) -> Result<String, Error> {
    Ok(path.to_module_specifier()?.to_string())
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
