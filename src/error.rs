use thiserror::Error;

use crate::Module;

/// Represents the errors that can occur during execution of a module
#[derive(Error, Debug)]
pub enum Error {
    /// Triggers when a module has no stated entrypoint (default or registered at runtime)
    #[error("{0} has no entrypoint. Register one, or add a default to the runtime")]
    MissingEntrypoint(Module),

    /// Triggers when an attempt to find a value by name fails
    #[error("{0} could not be found in global, or module exports")]
    ValueNotFound(String),

    /// Triggers when attempting to call a value as a function
    #[error("{0} is not a function")]
    ValueNotCallable(String),

    /// Triggers when a string could not be encoded for v8
    #[error("{0} could not be encoded as a v8 value")]
    V8Encoding(String),

    /// Triggers when a result could not be deserialize to the requested type
    #[error("value could not be deserialized: {0}")]
    JsonDecode(String),

    /// Triggers on runtime issues during execution of a module
    #[error("{0}")]
    Runtime(String),

    /// Triggers when a module times out before finishing
    #[error("Module timed out: {0}")]
    Timeout(String),
}

#[macro_use]
mod error_macro {
    /// Maps one error type to another
    macro_rules! map_error {
        ($source_error:path, $impl:expr) => {
            impl From<$source_error> for Error {
                fn from(e: $source_error) -> Self {
                    let fmt: &dyn Fn(&$source_error) -> Self = &$impl;
                    fmt(&e)
                }
            }
        };
    }
}

map_error!(std::cell::BorrowMutError, |e| Error::Runtime(e.to_string()));
map_error!(std::io::Error, |e| Error::Runtime(e.to_string()));
map_error!(deno_core::v8::DataError, |e| Error::Runtime(e.to_string()));
map_error!(deno_core::ModuleResolutionError, |e| Error::Runtime(
    e.to_string()
));
map_error!(deno_core::serde_json::Error, |e| Error::JsonDecode(
    e.to_string()
));
map_error!(deno_core::serde_v8::Error, |e| Error::JsonDecode(
    e.to_string()
));
map_error!(deno_core::anyhow::Error, |e| Error::Runtime(e.to_string()));
map_error!(tokio::time::error::Elapsed, |e| {
    Error::Timeout(e.to_string())
});
map_error!(deno_core::futures::channel::oneshot::Canceled, |e| {
    Error::Timeout(e.to_string())
});
