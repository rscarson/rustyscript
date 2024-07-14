///
/// This example shows a basic usage of the runtime
///
/// The call to Runtime::execute_module is a one liner which:
/// - Creates a new runtime with the given options
/// - Loads the modules passed in
/// - Calls the module's entrypoint function
///     - Either a default passed as an option
///     - Or the module's default export
///     - Or a call in JS to rustyscript.register_entrypoint
/// - Returns the result
///
/// Instead of just vec![], one could pass in other JS modules
/// which could be imported using `import './filename.js';`
///
use rustyscript::{Error, Module, Runtime};

fn main() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        "
        // Importing a module first loaded by rust
        import * as test2 from './test2.js';

        // Importing a module from the filesystem - requires 'web' or 'fs_import' crate feature
        import * as fsimport from './examples/javascript/example_module.js';

        // Importing a module from the web - requires 'web' or 'url_import' crate feature
        import * as json from 'https://deno.land/std@0.206.0/json/common.ts';
        ",
    );
    let module2 = Module::new("test2.js", "");

    let mut runtime = Runtime::new(Default::default())?;
    runtime.load_module(&module2)?;
    runtime.load_module(&module)?;

    Ok(())
}
