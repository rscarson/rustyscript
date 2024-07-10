use crate::Module;
use thiserror::Error;

/// Options for [Error::as_highlighted]
#[derive(Debug, Clone)]
pub struct ErrorFormattingOptions {
    pub include_filename: bool,
    pub include_line_number: bool,
    pub include_column_number: bool,
}
impl Default for ErrorFormattingOptions {
    fn default() -> Self {
        Self {
            include_filename: true,
            include_line_number: true,
            include_column_number: true,
        }
    }
}

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
    pub fn as_highlighted(&self, options: ErrorFormattingOptions) -> String {
        match self {
            Error::JsError(e) => {
                // Extract basic information about position
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

                let mut line = e.source_line.as_ref().map(|s| s.trim_end());
                let col = col - 1;

                // Get at most 50 characters, centered on column_number
                let mut padding = String::new();
                match line {
                    None => {}
                    Some(s) => {
                        let (start, end) = if s.len() < 50 {
                            (0, s.len())
                        } else if col < 25 {
                            (0, 50)
                        } else if col > s.len() - 25 {
                            (s.len() - 50, s.len())
                        } else {
                            (col - 25, col + 25)
                        };

                        line = Some(s.get(start..end).unwrap_or(s));
                        padding = " ".repeat(col - start - 1);
                    }
                }

                let msg_lines = e.exception_message.split('\n').collect::<Vec<_>>();

                //
                // Format all the parts using the options
                //

                let line_number_part = match options.include_line_number {
                    true => format!("{}:", row),
                    false => String::new(),
                };

                let col_number_part = match options.include_column_number {
                    true => format!("{}:", col),
                    false => String::new(),
                };

                let source_line_part = match line {
                    Some(s) => format!("| {s}\n| {padding}^\n"),
                    None => String::new(),
                };

                let msg_part = msg_lines
                    .into_iter()
                    .map(|l| format!("= {l}"))
                    .collect::<Vec<_>>()
                    .join("\n");

                let position_part = format!("{line_number_part}{col_number_part}");
                let position_part = match filename {
                    None if position_part.is_empty() => String::new(),
                    Some(f) if options.include_filename => format!("{f}:{position_part}\n"),
                    _ => format!("At {position_part}\n"),
                };

                // Combine all the parts
                format!("{position_part}{source_line_part}{msg_part}",)
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

#[cfg(test)]
mod test {
    use crate::{error::ErrorFormattingOptions, Module, Runtime, Undefined};

    #[test]
    #[rustfmt::skip]
    fn test_highlights() {
        let mut runtime = Runtime::new(Default::default()).unwrap();

        let e = runtime.eval::<Undefined>("1+1;\n1 + x").unwrap_err().as_highlighted(Default::default());
        assert_eq!(e, concat!(
            "At 2:1:\n",
            "| 1 + x\n",
            "| ^\n",
            "= Uncaught ReferenceError: x is not defined"
        ));

        let module = Module::new("test.js", "1+1;\n1 + x");
        let e = runtime.load_module(&module).unwrap_err().as_highlighted(ErrorFormattingOptions {
            include_filename: false,
            ..Default::default()
        });
        assert_eq!(e, concat!(
            "At 2:1:\n",
            "| 1 + x\n",
            "| ^\n",
            "= Uncaught (in promise) ReferenceError: x is not defined"
        ));
    }
}
