## Effortless JS Integration for Rust

[![Crates.io](https://img.shields.io/crates/v/rustyscript.svg)](https://crates.io/crates/rustyscript)
[![Build Status](https://github.com/rscarson/rustyscript/workflows/Rust/badge.svg)](https://github.com/rscarson/rustyscript/actions?workflow=Rust)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://raw.githubusercontent.com/rscarson/rustyscript/master/LICENSE)

<!-- cargo-rdme start -->

This crate is meant to provide a quick and simple way to integrate a runtime javacript or typescript component from within rust.

- **By default, the code being run is entirely sandboxed from the host, having no filesystem or network access.**
    - It can be extended to include those capabilities and more if desired - please see the 'web' feature, and the `runtime_extensions` example
- Asynchronous JS code is supported (I suggest using the timeout option when creating your runtime)
- Loaded JS modules can import other modules
- Typescript is supported by default, and will be transpiled into JS for execution

----

Here is a very basic use of this crate to execute a JS module. It will:
- Create a basic runtime
- Load a javascript module,
- Call a function registered as the entrypoint
- Return the resulting value
```rust
use rustyscript::{json_args, Runtime, Module, Error};

let module = Module::new(
    "test.js",
    "
    rustyscript.register_entrypoint(
        (string, integer) => {
            console.log(`Hello world: string=${string}, integer=${integer}`);
            return 2;
        }
    )
    "
);

let value: usize = Runtime::execute_module(
    &module, vec![],
    Default::default(),
    json_args!("test", 5)
)?;

assert_eq!(value, 2);
```

Modules can also be loaded from the filesystem with `Module::load` or `Module::load_dir` if you want to collect all modules in a given directory.

----

If all you need is the result of a single javascript expression, you can use:
```rust
let result: i64 = rustyscript::evaluate("5 + 5").expect("The expression was invalid!");
```

Or to just import a single module for use:
```rust
use rustyscript::{json_args, import};
let mut module = import("js/my_module.js").expect("Something went wrong!");
let value: String = module.call("exported_function_name", json_args!()).expect("Could not get a value!");
```

There are a few other utilities included, such as `rustyscript::validate` and `rustyscript::resolve_path`

----

A more detailed version of the crate's usage can be seen below, which breaks down the steps instead of using the one-liner `Runtime::execute_module`:
```rust
use rustyscript::{json_args, Runtime, RuntimeOptions, Module, Error, Undefined};
use std::time::Duration;

let module = Module::new(
    "test.js",
    "
    let internalValue = 0;
    export const load = (value) => internalValue = value;
    export const getValue = () => internalValue;
    "
);

// Create a new runtime
let mut runtime = Runtime::new(RuntimeOptions {
    timeout: Duration::from_millis(50), // Stop execution by force after 50ms
    default_entrypoint: Some("load".to_string()), // Run this as the entrypoint function if none is registered
    ..Default::default()
})?;

// The handle returned is used to get exported functions and values from that module.
// We then call the entrypoint function, but do not need a return value.
//Load can be called multiple times, and modules can import other loaded modules
// Using `import './filename.js'`
let module_handle = runtime.load_module(&module)?;
runtime.call_entrypoint::<Undefined>(&module_handle, json_args!(2))?;

// Functions don't need to be the entrypoint to be callable!
let internal_value: i64 = runtime.call_function(&module_handle, "getValue", json_args!())?;
```

Rust functions can also be registered to be called from javascript:
```rust
use rustyscript::{ Runtime, Module, serde_json::Value };

let module = Module::new("test.js", " rustyscript.functions.foo(); ");
let mut runtime = Runtime::new(Default::default())?;
runtime.register_function("foo", |args, _state| {
    if let Some(value) = args.get(0) {
        println!("called with: {}", value);
    }
    Ok(Value::Null)
})?;
runtime.load_module(&module)?;
```

For better performance calling rust code, consider using an extension instead - see the `runtime_extensions` example for details

The 'state' parameter can be used to persist data - please see the `call_rust_from_js` example for details

----

#### Utility Functions
These functions provide simple one-liner access to common features of this crate:
- evaluate; Evaluate a single JS expression and return the resulting value
- import; Get a handle to a JS module from which you can get exported values and functions
- resolve_path; Resolve a relative path to the current working dir
- validate; Validate the syntax of a JS expression

#### Crate features
The table below lists the available features for this crate. Features marked at `Preserves Sandbox: NO` break isolation between loaded JS modules and the host system.
Use with caution.

| Feature     | Description                                                                                       | Preserves Sandbox | Dependencies                                                        |  
|-------------|---------------------------------------------------------------------------------------------------|------------------|----------------------------------------------------------------------|
|console      |Provides `console.*` functionality from JS                                                         |yes               |deno_console                                                          |
|crypto       |Provides `crypto.*` functionality from JS                                                          |yes               |deno_crypto, deno_webidl                                              |
|url          |Provides the URL, and URLPattern APIs from within JS                                               |yes               |deno_webidl, deno_url                                                 |
|web          |Provides the Event, TextEncoder, TextDecoder, File, Web Cryptography, and fetch APIs from within JS|**NO**            |deno_webidl, deno_web, deno_crypto, deno_fetch, deno_url              |
|             |                                                                                                   |                  |                                                                      |
|default      |Provides only those extensions that preserve sandboxing                                            |yes               |deno_console, deno_crypto, deno_webidl, deno_url                      |
|no_extensions|Disables all extensions to the JS runtime - you can still add your own extensions in this mode     |yes               |None                                                                  |
|all          |Provides all available functionality                                                               |**NO**            |deno_console, deno_webidl, deno_web, deno_crypto, deno_fetch, deno_url|
|             |                                                                                                   |                  |                                                                      |
|fs_import    | Enables importing arbitrary code from the filesystem through JS                                   |**NO**            |None                                                                  |
|url_import   | Enables importing arbitrary code from network locations through JS                                |**NO**            |reqwest                                                               |
----

Please also check out [@Bromeon/js_sandbox](https://github.com/Bromeon/js-sandbox), another great crate in this niche

For an example of this crate in use, please check out [lavendeux-parser](https://github.com/rscarson/lavendeux-parser)

<!-- cargo-rdme end -->
