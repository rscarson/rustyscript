use crate::Module;
use thiserror::Error;

/// Represents the errors that can occur during execution of a module
#[derive(Error, Debug, Clone, serde::Serialize, serde::Deserialize)]
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

    /// Triggers when a module could not be loaded from the filesystem
    #[error("{0}")]
    ModuleNotFound(String),

    /// Triggers on runtime issues during execution of a module
    #[error("{0}")]
    Runtime(String),

    /// Runtime error we successfully downcast
    #[error("{0}")]
    JsError(#[from] deno_core::error::JsError),

    /// Triggers when a module times out before finishing
    #[error("Module timed out: {0}")]
    Timeout(String),
}

impl Error {
    /// Formats an error for display in a terminal
    /// If the error is a JsError, it will attempt to highlight the source line
    /// in this format:
    /// ```text
    /// | let x = 1 + 2
    /// |       ^
    /// = Unexpected token '='
    /// ```
    ///
    /// Otherwise, it will just display the error message normally
    pub fn as_highlighted(&self) -> String {
        match self {
            Error::JsError(e) if e.source_line.is_some() => {
                let (filename, row, col) = match e.frames.first() {
                    Some(f) => (
                        match &f.file_name {
                            Some(f) if f.is_empty() => None::<&str>,
                            Some(f) => Some(f.as_ref()),
                            None => None,
                        },
                        f.line_number.unwrap_or(1) as usize,
                        f.line_number.unwrap_or(1) as usize,
                    ),
                    None => (None, 1, 1),
                };

                let line = e.source_line.as_ref().unwrap();
                let line = line.trim_end();
                let col = col - 1;

                // Get at most 50 characters, centered on column_number
                let (start, end) = if line.len() < 50 {
                    (0, line.len())
                } else if col < 25 {
                    (0, 50)
                } else if col > line.len() - 25 {
                    (line.len() - 50, line.len())
                } else {
                    (col - 25, col + 25)
                };

                let line = line.get(start..end).unwrap_or(line);
                let fpos = if let Some(filename) = filename {
                    format!("{}:{}\n", filename, row)
                } else if row > 1 {
                    format!("Line {}\n", row)
                } else {
                    "".to_string()
                };

                let msg = e
                    .exception_message
                    .split('\n')
                    .map(|l| format!("= {}", l))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{fpos}| {line}\n| {}^\n{msg}", " ".repeat(col - start))
            }
            _ => format!("{}", self),
        }
    }
}

#[macro_use]
mod error_macro {
    /// Maps one error type to another
    macro_rules! map_error {
        ($source_error:path, $impl:expr) => {
            impl From<$source_error> for Error {
                fn from(e: $source_error) -> Self {
                    let fmt: &dyn Fn($source_error) -> Self = &$impl;
                    fmt(e)
                }
            }
        };
    }
}

map_error!(std::cell::BorrowMutError, |e| Error::Runtime(e.to_string()));
map_error!(std::io::Error, |e| Error::ModuleNotFound(e.to_string()));
map_error!(deno_core::v8::DataError, |e| Error::Runtime(e.to_string()));
map_error!(deno_core::ModuleResolutionError, |e| Error::Runtime(
    e.to_string()
));
map_error!(deno_core::url::ParseError, |e| Error::Runtime(
    e.to_string()
));
map_error!(deno_core::serde_json::Error, |e| Error::JsonDecode(
    e.to_string()
));
map_error!(deno_core::serde_v8::Error, |e| Error::JsonDecode(
    e.to_string()
));

map_error!(deno_core::anyhow::Error, |e| {
    // trydowncast to deno_core::error::JsError
    let s = e.to_string();
    match e.downcast::<deno_core::error::JsError>() {
        Ok(js_error) => Error::JsError(js_error),
        Err(_) => Error::Runtime(s),
    }
});

map_error!(tokio::time::error::Elapsed, |e| {
    Error::Timeout(e.to_string())
});
map_error!(tokio::task::JoinError, |e| {
    Error::Timeout(e.to_string())
});
map_error!(deno_core::futures::channel::oneshot::Canceled, |e| {
    Error::Timeout(e.to_string())
});
