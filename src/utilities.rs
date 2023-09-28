use crate::{ Script, Runtime, Error};

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
where T: deno_core::serde::de::DeserializeOwned {
    let script = Script::new("js_eval.js", &format!("
        export function js_playground_evaluate(){{
            return ({javascript});
        }}
    "));
    let mut runtime = Runtime::new(Default::default())?;
    let module = runtime.load_modules(script, vec![])?;
    runtime.call_function(&module, "js_playground_evaluate", Runtime::EMPTY_ARGS)
}

#[cfg(test)]
mod test_runtime {
    use super::*;
    
    #[test]
    fn test_evaluate() {
        assert_eq!(5, evaluate::<i64>("3 + 2").expect("invalid expression"));
        evaluate::<i64>("5; 3 + 2").expect_err("invalid expression");
    }
}