//! # Simple deno wrapper for module execution
//!
//! [![Crates.io](https://img.shields.io/crates/v/js-playground.svg)](https://crates.io/crates/js-playground)
//! [![Build Status](https://github.com/rscarson/js-playground/workflows/Rust/badge.svg)](https://github.com/rscarson/js-playground/actions?workflow=Rust)
//! [![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://raw.githubusercontent.com/rscarson/js-playground/master/LICENSE)
//!
//! This crate is meant to provide a quick and simple way to integrate a runtime JS or TS component from within rust.
//! By default, the code being run is entirely sandboxed from the host, having no filesystem or network access.
//!
//! Typescript is supported by default
//!
//! It can be extended to include the capabilities and more if desired - please see the `runtime_extensions` example
//!
//! Asynchronous code is supported - I suggest using the timeout option when creating your runtime to avoid infinite hangs.
//!
//! Modules loaded can be imported by other code, and `reset()` can be called to unload modules and reset the global object.
//!
//! Here is a very basic use of this crate to execute a JS module. It will create a basic runtime, load the script,
//! call the registered entrypoint function with the given arguments, and return the resulting value:
//! ```rust
//! use js_playground::{Runtime, Script, Error};
//!
//! # fn main() -> Result<(), Error> {
//! let script = Script::new(
//!     "test.js",
//!     "
//!     js_playground.register_entrypoint(
//!         (string, integer) => {
//!             console.log(`Hello world: string=${string}, integer=${integer}`);
//!             return 2;
//!         }
//!     )
//!     "
//! );
//!
//! let value: usize = Runtime::execute_module(
//!     &script, vec![],
//!     Default::default(),
//!     &[
//!         Runtime::arg("test"),
//!         Runtime::arg(5),
//!     ]
//! )?;
//!
//! assert_eq!(value, 2);
//! # Ok(())
//! # }
//! ```
//!
//! Scripts can also be loaded from the filesystem with `Script::load` or `Script::load_dir` if you want to collect all modules in a given directory.
//!
//! If all you need is the result of a single javascript expression, you can use:
//! ```rust
//! let result: i64 = js_playground::evaluate("5 + 5").expect("The expression was invalid!");
//! ```
//!
//! There are a few other utilities included, such as `js_playground::validate` and `js_playground::resolve_path`
//!
//! A more detailed version of the crate's usage can be seen below, which breaks down the steps instead of using the one-liner `Runtime::execute_module`:
//! ```rust
//! use js_playground::{Runtime, RuntimeOptions, Script, Error, Undefined};
//! use std::time::Duration;
//!
//! # fn main() -> Result<(), Error> {
//! let script = Script::new(
//!     "test.js",
//!     "
//!     let internalValue = 0;
//!     export const load = (value) => internalValue = value;
//!     export const getValue = () => internalValue;
//!     "
//! );
//!
//! // Create a new runtime
//! let mut runtime = Runtime::new(RuntimeOptions {
//!     timeout: Some(Duration::from_millis(50)), // Stop execution by force after 50ms
//!     default_entrypoint: Some("load".to_string()), // Run this as the entrypoint function if none is registered
//!     ..Default::default()
//! })?;
//!
//! // The handle returned is used to get exported functions and values from that module.
//! // We then call the entrypoint function, but do not need a return value.
//! let module_handle = runtime.load_module(&script)?;
//! runtime.call_entrypoint::<Undefined>(&module_handle, &[ Runtime::arg(2) ])?;
//!
//! let internal_value: i64 = runtime.call_function(&module_handle, "getValue", Runtime::EMPTY_ARGS)?;
//! # Ok(())
//! # }
//! ```
//!
//! Please also check out [@Bromeon/js_sandbox](https://github.com/Bromeon/js-sandbox), another great crate in this niche
//!
#![warn(missing_docs)]

mod error;
mod ext;
mod inner_runtime;
mod module_handle;
mod runtime;
mod script;
mod traits;
mod utilities;

// Expose a few dependencies that could be useful
pub use deno_core;
pub use deno_core::serde_json;

// Expose some important stuff from us
pub use error::Error;
pub use module_handle::ModuleHandle;
pub use runtime::{Runtime, RuntimeOptions, Undefined};
pub use script::{Script, StaticScript};
pub use utilities::{evaluate, resolve_path, validate};
