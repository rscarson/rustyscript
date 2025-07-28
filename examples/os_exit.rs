///
/// This example shows how to use the os_exit feature to provide
/// os.exit functionality to JavaScript code.
///
/// The os_exit feature enables the deno_os extension which provides
/// process termination capabilities through Deno.exit().
///
/// Note: This example doesn't actually call exit() as that would
/// terminate the process. Instead, it demonstrates that the 
/// functionality is available.
///
use rustyscript::{Error, Module, Runtime, RuntimeOptions};

fn main() -> Result<(), Error> {
    let module = Module::new(
        "test_exit.js",
        r#"
        // Check if Deno.exit is available
        if (typeof Deno !== 'undefined' && typeof Deno.exit === 'function') {
            console.log("✓ Deno.exit is available");
            
            // We can test the function exists but won't call it
            // as that would terminate this example program
            console.log("  Function signature:", Deno.exit.toString());
        } else {
            console.log("✗ Deno.exit is not available");
            console.log("  Make sure to compile with --features=\"os_exit\"");
        }
        
        export const hasExit = typeof Deno?.exit === 'function';
        "#,
    );

    let mut runtime = Runtime::new(RuntimeOptions::default())?;
    let module_handle = runtime.load_module(&module)?;

    let has_exit: bool = runtime.get_value(Some(&module_handle), "hasExit")?;
    
    if has_exit {
        println!("Success! The os_exit feature is working correctly.");
        println!("JavaScript code now has access to Deno.exit() for process termination.");
    } else {
        println!("The os_exit feature is not enabled.");
        println!("Try running: cargo run --example os_exit --features=\"os_exit\"");
    }

    Ok(())
}