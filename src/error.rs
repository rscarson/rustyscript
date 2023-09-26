use std::fmt::{ Display, Formatter, Result };
use deno_core::anyhow;
use crate::Script;

#[macro_use]
mod error_macro {
    macro_rules! define_error {
        ($(
            name = $name:ident($($param:ident:$type:path),+),
            docs = $docs:literal,
            formatter = $formatter_closure:expr
        ),+) => {
            $(
                #[doc = $docs]
                #[derive(Debug)]
                pub struct $name($($type,)+);
                impl $name {
                    pub fn new($($param:$type,)+) -> Self {
                        Self($($param,)+)
                    }
                }
                impl std::error::Error for $name {}
                impl Display for $name {
                    fn fmt(&self, f: &mut Formatter) -> Result {
                        let fmt: &dyn Fn(&Self) -> String = &$formatter_closure;
                        write!(f, "{}: {}", stringify!(name), fmt(self))
                    }
                }
            )+
            
            /// An error occuring as a result of js_playground
            #[derive(Debug)]
            pub enum Error {
                $(
                    #[doc = $docs]
                    $name($name),
                )+
            }
            impl std::error::Error for Error {}
            impl Display for Error {
                fn fmt(&self, f: &mut Formatter) -> Result {
                    match self {  
                        $(
                            Self::$name(e) => write!(f, "{}", e),
                        )+
                    }
                }
            }
            $(
                map_error!($name);
            )+
        }
    }

    macro_rules! map_error {
        ($source_error:ident) => {
            impl From<$source_error> for Error {
                fn from(e: $source_error) -> Error {
                    Self::$source_error(e.into())
                }
            }
        };
        ($source_error:path, $target_error:ident, $impl:expr) => {
            impl From<$source_error> for Error {
                fn from(e: $source_error) -> Error {
                    Self::$target_error(e.into())
                }
            }
            impl From<$source_error> for $target_error {
                fn from(e: $source_error) -> $target_error {
                    let fmt: &dyn Fn(&$source_error) -> $target_error = &$impl;
                    Self(fmt(&e).into())
                }
            }
        };
    }
}

define_error!(
    name = MissingEntrypointError(module: Script),
    docs = "Triggers when a module has no stated entrypoint (default or registered at runtime)",
    formatter = |this| format!("{} has no entrypoint. Register one, or add a default to the runtime", this.0.filename()),
    
    name = ValueNotFoundError(name: String),
    docs = "Triggers when an attempt to find a value by name fails",
    formatter = |this| format!("{} could not be found in global, or module exports", this.0),
    
    name = ValueNotCallableError(name: String),
    docs = "Triggers when attempting to call a value as a function",
    formatter = |this| format!("{} is not a function", this.0),
    
    name = V8EncodingError(source: String),
    docs = "Triggers when a string could not be encoded for v8",
    formatter = |this| format!("'{}' could not be encoded as a v8 value", this.0),
    
    name = JsonDecodeError(cause: anyhow::Error),
    docs = "Triggers when a result could not be deserialize to the requested type",
    formatter = |this| format!("value could not be deserialized: {}", this.0),
    
    name = RuntimeError(cause: anyhow::Error),
    docs = "Triggers on runtime issues during execution of a script",
    formatter = |this| format!("{}", this.0)
);

map_error!(std::cell::BorrowMutError, RuntimeError, |e| RuntimeError(e.into()));
map_error!(std::io::Error, RuntimeError, |e| RuntimeError(e.into()));
map_error!(deno_core::v8::DataError, RuntimeError, |e| RuntimeError(e.into()));
map_error!(deno_core::ModuleResolutionError, RuntimeError, |e| RuntimeError(e.into()));
map_error!(deno_core::serde_json::Error, JsonDecodeError, |e| JsonDecodeError(e.into()));
map_error!(deno_core::serde_v8::Error, JsonDecodeError, |e| JsonDecodeError(e.into()));

map_error!(deno_core::anyhow::Error, RuntimeError, |e| RuntimeError(*e));