## Effortless JS Integration for Rust

[![Crates.io](https://img.shields.io/crates/v/rustyscript.svg)](https://crates.io/crates/rustyscript)
[![Build Status](https://github.com/rscarson/rustyscript/actions/workflows/tests.yml/badge.svg?branch=master)](https://github.com/rscarson/rustyscript/actions?query=branch%3Amaster)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://raw.githubusercontent.com/rscarson/rustyscript/master/LICENSE)

<!-- cargo-rdme start -->

This crate is meant to provide a quick and simple way to integrate a runtime javascript or typescript component from within rust.

It uses the v8 engine through the `deno_core` crate, and is meant to be as simple as possible to use without sacrificing flexibility or performance.

I also have attempted to abstract away the v8 engine details so you can for the most part operate directly on rust types.

- **By default, the code being run is entirely sandboxed from the host, having no filesystem or network access.**
    - It can be extended to include those capabilities and more if desired - please see the `web` feature, and the `runtime_extensions` example
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
    export default (string, integer) => {
        console.log(`Hello world: string=${string}, integer=${integer}`);
        return 2;
    }
    "
);

let value: usize = Runtime::execute_module(
    &module, vec![],
    Default::default(),
    json_args!("test", 5)
)?;

assert_eq!(value, 2);
```

Modules can also be loaded from the filesystem with [`Module::load`] or [`Module::load_dir`] if you want to collect all modules in a given directory.

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

There are a few other utilities included, such as [`validate`] and [`resolve_path`]

----

A more detailed version of the crate's usage can be seen below, which breaks down the steps instead of using the one-liner [`Runtime::execute_module`]:
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
let internal_value: i64 = runtime.call_function(Some(&module_handle), "getValue", json_args!())?;
```

There are also '_async' and 'immediate' versions of most runtime functions;
'_async' functions return a future that resolves to the result of the operation, while
'_immediate' functions will make no attempt to wait for the event loop, making them suitable
for using [`crate::js_value::Promise`]

Rust functions can also be registered to be called from javascript:
```rust
use rustyscript::{ Runtime, Module, serde_json::Value };

let module = Module::new("test.js", " rustyscript.functions.foo(); ");
let mut runtime = Runtime::new(Default::default())?;
runtime.register_function("foo", |args| {
    if let Some(value) = args.get(0) {
        println!("called with: {}", value);
    }
    Ok(Value::Null)
})?;
runtime.load_module(&module)?;
```

----

Asynchronous JS can be called in 2 ways;

The first is to use the 'async' keyword in JS, and then call the function using [`Runtime::call_function_async`]
```rust
use rustyscript::{ Runtime, Module, json_args };

let module = Module::new("test.js", "export async function foo() { return 5; }");
let mut runtime = Runtime::new(Default::default())?;

// The runtime has its own tokio runtime; you can get a handle to it with [Runtime::tokio_runtime]
// You can also build the runtime with your own tokio runtime, see [Runtime::with_tokio_runtime]
let tokio_runtime = runtime.tokio_runtime();

let result: i32 = tokio_runtime.block_on(async {
    // Top-level await is supported - we can load modules asynchronously
    let handle = runtime.load_module_async(&module).await?;

    // Call the function asynchronously
    runtime.call_function_async(Some(&handle), "foo", json_args!()).await
})?;

assert_eq!(result, 5);
```

The second is to use [`crate::js_value::Promise`]
```rust
use rustyscript::{ Runtime, Module, js_value::Promise, json_args };

let module = Module::new("test.js", "export async function foo() { return 5; }");

let mut runtime = Runtime::new(Default::default())?;
let handle = runtime.load_module(&module)?;

// We call the function without waiting for the event loop to run, or for the promise to resolve
// This way we can store it and wait for it later, without blocking the event loop or borrowing the runtime
let result: Promise<i32> = runtime.call_function_immediate(Some(&handle), "foo", json_args!())?;

// We can then wait for the promise to resolve
// We can do so asynchronously, using [crate::js_value::Promise::into_future]
// But we can also block the current thread:
let result = result.into_value(&mut runtime)?;
assert_eq!(result, 5);
```

- See [`Runtime::register_async_function`] for registering and calling async rust from JS
- See `examples/async_javascript.rs` for a more detailed example of using async JS

----

For better performance calling rust code, consider using an extension instead of a module - see the `runtime_extensions` example for details

----

A threaded worker can be used to run code in a separate thread, or to allow multiple concurrent runtimes.

the [`worker`] module provides a simple interface to create and interact with workers.
The [`worker::InnerWorker`] trait can be implemented to provide custom worker behavior.

It also provides a default worker implementation that can be used without any additional setup:
```rust
use rustyscript::{Error, worker::{Worker, DefaultWorker, DefaultWorkerOptions}};
use std::time::Duration;

fn main() -> Result<(), Error> {
    let worker = DefaultWorker::new(DefaultWorkerOptions {
        default_entrypoint: None,
        timeout: Duration::from_secs(5),
    })?;

    let result: i32 = worker.eval("5 + 5".to_string())?;
    assert_eq!(result, 10);
    Ok(())
}
```

----

#### Utility Functions
These functions provide simple one-liner access to common features of this crate:
- `evaluate`; Evaluate a single JS expression and return the resulting value
- `import`; Get a handle to a JS module from which you can get exported values and functions
- `resolve_path`; Resolve a relative path to the current working dir
- `validate`; Validate the syntax of a JS expression
- `init_platform`; Initialize the V8 platform for multi-threaded applications

Commonly used features have been grouped into the following feature-sets:
- **`safe_extensions`** - On by default, these extensions are safe to use in a sandboxed environment
- **`network_extensions`** - These extensions break sandboxing by allowing network connectivity
- **`io_extensions`** - These extensions break sandboxing by allowing filesystem access (WARNING: Also allows some network access)
- **`all_extensions`** - All 3 above groups are included
- **`extra_features`** - Enables the `worker` feature (enabled by default), and the `snapshot_builder` feature
- **`node_experimental`** - HIGHLY EXPERIMENTAL nodeJS support that enables all available Deno extensions

#### Crate features
The table below lists the available features for this crate. Features marked at `Preserves Sandbox: NO` break isolation between loaded JS modules and the host system.
Use with caution.

More details on the features can be found in `Cargo.toml`

Please note that the `web` feature will also enable `fs_import` and `url_import`, allowing arbitrary filesystem and network access for import statements
- This is because the `deno_web` crate allows both fetch and FS reads already

| Feature           | Description                                                                                               | Preserves Sandbox| Dependencies                                                                                  |  
|-------------------|-----------------------------------------------------------------------------------------------------------|------------------|-----------------------------------------------------------------------------------------------|
|`broadcast_channel`|Implements the web-messaging API for Deno                                                                  |**NO**            |`deno_broadcast_channel`, `deno_web`, `deno_webidl`                                            |
|`cache`            |Implements the Cache API for Deno                                                                          |**NO**            |`deno_cache`, `deno_webidl`, `deno_web`, `deno_crypto`, `deno_fetch`, `deno_url`, `deno_net`   |
|`console`          |Provides `console.*` functionality from JS                                                                 |yes               |`deno_console`, `deno_terminal`                                                                |
|`cron`             |Implements scheduled tasks (crons) API                                                                     |**NO**            |`deno_cron`, `deno_console`                                                                    |
|`crypto`           |Provides `crypto.*` functionality from JS                                                                  |yes               |`deno_crypto`, `deno_webidl`                                                                   |
|`ffi`              |Dynamic library ffi features                                                                               |**NO**            |`deno_ffi`                                                                                     |
|`fs`               |Provides ops for interacting with the file system.                                                         |**NO**            |`deno_fs`, `web`,  `io`                                                                        |
|`http`             |Implements the fetch standard                                                                              |**NO**            |`deno_http`, `web`, `websocket`                                                                |
|`kv`               |Implements the Deno KV Connect protocol                                                                    |**NO**            |`deno_kv`, `web`, `console`                                                                    |
|`url`              |Provides the `URL`, and `URLPattern` APIs from within JS                                                   |yes               |`deno_webidl`, `deno_url`                                                                      |
|`io`               |Provides IO primitives such as stdio streams and abstraction over File System files.                       |**NO**            |`deno_io`, `rustyline`, `winapi`, `nix`, `libc`, `once_cell`                                   |
|`web`              |Provides the `Event`, `TextEncoder`, `TextDecoder`, `File`, Web Cryptography, and fetch APIs from within JS|**NO**            |`deno_webidl`, `deno_web`, `deno_crypto`, `deno_fetch`, `deno_url`, `deno_net`                 |
|`webgpu`           |Implements the WebGPU API                                                                                  |**NO**            |`deno_webgpu`, `web`                                                                           |
|`webstorage`       |Provides the `WebStorage` API                                                                              |**NO**            |`deno_webidl`, `deno_webstorage`                                                               |
|`websocket`        |Provides the `WebSocket` API                                                                               |**NO**            |`deno_web`, `deno_websocket`                                                                   |
|`webidl`           |Provides the `webidl` API                                                                                  |yes               |`deno_webidl`                                                                                  |
|                   |                                                                                                           |                  |                                                                                               |
|`default`          |Provides only those extensions that preserve sandboxing                                                    |yes               |`deno_console`, `deno_crypto`, `deno_webidl`, `deno_url`                                       |
|`no_extensions`    |Disables all extensions to the JS runtime - you can still add your own extensions in this mode             |yes               |None                                                                                           |
|`all`              |Provides all available functionality                                                                       |**NO**            |`deno_console`, `deno_webidl`, `deno_web`, `deno_net`, `deno_crypto`, `deno_fetch`, `deno_url` |
|                   |                                                                                                           |                  |                                                                                               |
|`fs_import`        |Enables importing arbitrary code from the filesystem through JS                                            |**NO**            |None                                                                                           |
|`url_import`       |Enables importing arbitrary code from network locations through JS                                         |**NO**            |`reqwest`                                                                                      |
|                   |                                                                                                           |                  |                                                                                               |
|`node_experimental`|HIGHLY EXPERIMENTAL nodeJS support that enables all available Deno extensions                              |**NO**            |For complete list, see Cargo.toml                                                              |
|                   |                                                                                                           |                  |                                                                                               |
|`worker`           |Enables access to the threaded worker API [`worker`]                                                       |yes               |None                                                                                           |
|`snapshot_builder` |Enables access to [`SnapshotBuilder`], a runtime for creating snapshots that can improve start-times       |yes               |None                                                                                           |
|`web_stub`         |Enables a subset of `web` features that do not break sandboxing                                            |yes               |`deno_webidl`                                                                                  |

----

For an example of this crate in use, see [Lavendeux](https://github.com/rscarson/lavendeux)

Please also check out [@Bromeon/js_sandbox](https://github.com/Bromeon/js-sandbox), another great crate in this niche

<!-- cargo-rdme end -->
