#[cfg(test)]
mod tests {
    use crate::{Runtime, RuntimeOptions, Module, Error};

    #[test]
    fn test_os_exit_extension_available() -> Result<(), Error> {
        // Test that the os.exit extension works correctly
        let mut runtime = Runtime::new(RuntimeOptions::default())?;
        
        let module = Module::new(
            "test_os_exit.js",
            r#"
            // Check if Deno.exit function is available
            export const has_deno = typeof Deno !== 'undefined';
            export const has_exit = typeof Deno?.exit === 'function';
            
            // Test the function works by checking it's callable
            // (We can't actually call it as it would terminate the test)
            export const is_callable = typeof Deno?.exit === 'function';
            
            // Test parameter validation without calling exit
            let param_validation = false;
            if (typeof Deno?.exit === 'function') {
                try {
                    // This should throw TypeError for non-integer
                    Deno.exit("invalid");
                } catch (e) {
                    param_validation = e instanceof TypeError;
                }
            }
            
            export const validation_works = param_validation;
            "#,
        );
        
        let handle = runtime.load_module(&module)?;
        
        // Verify that Deno object exists
        let has_deno: bool = runtime.get_value(Some(&handle), "has_deno")?;
        assert!(has_deno, "Deno object should be available");
        
        // Verify that exit function exists
        let has_exit: bool = runtime.get_value(Some(&handle), "has_exit")?;
        assert!(has_exit, "Deno.exit should be available with os_exit feature");
        
        // Verify that the function is callable
        let is_callable: bool = runtime.get_value(Some(&handle), "is_callable")?;
        assert!(is_callable, "Deno.exit should be a callable function");
        
        // Verify that parameter validation works
        let validation_works: bool = runtime.get_value(Some(&handle), "validation_works")?;
        assert!(validation_works, "Deno.exit should validate parameters correctly");
        
        Ok(())
    }
}