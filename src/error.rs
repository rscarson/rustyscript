use std::fmt::{ Display, Formatter, Result };

use deno_core::anyhow;

#[macro_use]
mod error_macro {
    macro_rules! define_error {
        ($(($name:ident, $docs:expr)),+) => {
            #[derive(Debug)]
            /// Represents an error occuring an any stage of module execution
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
                #[doc = $docs]
                #[derive(Debug)]
                pub struct $name(pub anyhow::Error);
                impl $name {
                    /// Get an instance of this error with the cause given by the string s
                    pub fn new_from_string(s: &str) -> Self {
                        Self(anyhow::Error::msg(s.to_string()))
                    }
                }
                impl std::error::Error for $name {}
                impl Display for $name {
                    fn fmt(&self, f: &mut Formatter) -> Result {
                        write!(f, "{}", self.0)
                    }
                }
                map_error!($name);
                map_error_variant!(anyhow::Error, $name);
            )+
            
            impl From<anyhow::Error> for Error {
                fn from(e: anyhow::Error) -> Error {
                    Self::RuntimeError(e.into())
                }
            }
        };
    }

    macro_rules! map_error {
        ($source_error:ident) => {
            impl From<$source_error> for Error {
                fn from(e: $source_error) -> Error {
                    Self::$source_error(e.into())
                }
            }
        };
        ($source_error:path, $target_error:ident) => {
            impl From<$source_error> for Error {
                fn from(e: $source_error) -> Error {
                    Self::$target_error(e.into())
                }
            }
            impl From<$source_error> for $target_error {
                fn from(e: $source_error) -> $target_error {
                    Self(e.into())
                }
            }
        };
    }

    macro_rules! map_error_variant {
        ($source_error:path, $target_error:ident) => {
            impl From<$source_error> for $target_error {
                fn from(e: $source_error) -> $target_error {
                    Self(e.into())
                }
            }
        };
    }
}

// Define the error types   
define_error!(
    (MissingEntrypointError, "Triggers when a module has no stated entrypoint (default or registered at runtime)"),
    (FunctionNotFoundError, "Triggers when an attempt to find a function by name fails"),
    (JsonDecodeError, "Triggers when a result could not be deserialize to the requested type"),
    (RuntimeError, "Triggers on runtime issues during execution of a script")
);

// A few other conversions we can do
map_error!(std::cell::BorrowMutError, RuntimeError);
map_error!(std::io::Error, RuntimeError);
map_error!(deno_core::v8::DataError, RuntimeError);
map_error!(deno_core::ModuleResolutionError, RuntimeError);
map_error!(deno_core::serde_json::Error, JsonDecodeError);
map_error!(deno_core::serde_v8::Error, JsonDecodeError);