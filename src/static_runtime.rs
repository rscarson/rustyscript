//! This module contains a macro for creating a static runtime instance
//! It creates a safe, thread-local runtime static.
//!
//! Can be used with default `RuntimeOptions` like so:
//! ```rust
//! use rustyscript::{RuntimeOptions, Error, static_runtime};
//! use std::time::Duration;
//!
//! static_runtime!(MY_DEFAULT_RUNTIME);
//!
//! fn main() -> Result<(), Error> {
//!     MY_DEFAULT_RUNTIME::with(|runtime| {
//!         runtime.eval::<()>("console.log('Hello, world!')")
//!     })
//! }
//! ```
//!
//! Or with custom `RuntimeOptions`:
//! ```rust
//! use rustyscript::{Error, RuntimeOptions, static_runtime};
//! use std::time::Duration;
//!
//! static_runtime!(MY_CUSTOM_RUNTIME, {
//!    RuntimeOptions {
//!        timeout: Duration::from_secs(5),
//!        ..Default::default()
//!    }
//! });
//!
//! fn main() -> Result<(), Error> {
//!     MY_CUSTOM_RUNTIME::with(|runtime| {
//!         runtime.eval::<()>("console.log('Hello, world!')")
//!     })
//! }
//! ```
use crate::{Error, Runtime, RuntimeOptions};
use std::cell::{OnceCell, RefCell, RefMut};

/// A lock for the static runtime
/// Created using `StaticRuntime::lock`
/// Should be mutable to be of any use
pub struct StaticRuntimeLock<'a> {
    lock: RefMut<'a, Result<Runtime, Error>>,
}
impl<'a> StaticRuntimeLock<'a> {
    /// Get a mutable reference to the runtime instance the lock is holding
    pub fn runtime(&mut self) -> &mut Runtime {
        match self.lock.as_mut() {
            Ok(rt) => rt,
            // Safety: The StaticRuntime::lock method ensures that we only get a lock if the runtime is initialized
            Err(_) => unreachable!("Could not get runtime lock"),
        }
    }
}

/// A static runtime instance
/// Uses `OnceCell` to ensure that the runtime is only initialized once
/// And `RefCell` to ensure that the runtime is never accessed concurrently
/// And finally, a `Result` to catch initialization errors.
///
/// Should be used with the `static_runtime!` macro
///
/// # Example
/// Can be used with default `RuntimeOptions` like so:
/// ```rust
/// use rustyscript::{Error, static_runtime};
///
/// static_runtime!(MY_DEFAULT_RUNTIME);
///
/// fn main() -> Result<(), Error> {
///     MY_DEFAULT_RUNTIME::with(|runtime| {
///         runtime.eval::<()>("console.log('Hello, world!')")
///     })
/// }
/// ```
///
/// Or with custom `RuntimeOptions`:
/// ```rust
/// use rustyscript::{Error, RuntimeOptions, static_runtime};
/// use std::time::Duration;
///
/// static_runtime!(MY_CUSTOM_RUNTIME, {
///    RuntimeOptions {
///        timeout: Duration::from_secs(5),
///        ..Default::default()
///    }
/// });
///
/// fn main() -> Result<(), Error> {
///     MY_CUSTOM_RUNTIME::with(|runtime| {
///         runtime.eval::<()>("console.log('Hello, world!')")
///     })
/// }
/// ```
pub struct StaticRuntime {
    init_options: fn() -> RuntimeOptions,
    cell: OnceCell<RefCell<Result<Runtime, Error>>>,
}
impl StaticRuntime {
    /// Create a new static runtime instance
    ///
    /// WARNING: This method should not be used directly, use the `static_runtime!` macro instead  
    /// Using this function will not encase the runtime in a `thread_local`, making it potentially unsafe
    pub const fn new(init_options: fn() -> RuntimeOptions) -> Self {
        Self {
            init_options,
            cell: OnceCell::new(),
        }
    }

    /// Get a reference to the runtime instance
    fn cell_ref(&self) -> &RefCell<Result<Runtime, Error>> {
        self.cell
            .get_or_init(|| RefCell::new(Runtime::new((self.init_options)())))
    }

    /// Get a lock for the runtime instance
    /// Will return a `StaticRuntimeLock` if the runtime is initialized
    /// You can then use `StaticRuntimeLock::runtime` to get a mutable reference to the runtime
    ///
    /// # Errors
    /// Will return an error if the runtime cannot be started (usually due to extension issues)
    pub fn lock(&self) -> Result<StaticRuntimeLock<'_>, Error> {
        let rt_mut = self.cell_ref();

        // Safety: We only get a lock if the runtime is initialized
        if let Err(e) = rt_mut.borrow_mut().as_ref() {
            return Err(Error::Runtime(format!(
                "Could not initialize static runtime: {e}"
            )));
        }

        Ok(StaticRuntimeLock {
            lock: rt_mut.borrow_mut(),
        })
    }

    /// Perform an operation on the runtime instance
    /// Will return T if we can get access to the runtime
    ///
    /// # Arguments
    /// * `callback` - A closure that takes a mutable reference to the runtime
    ///
    /// # Errors
    /// Will return an error if the runtime cannot be started (usually due to extension issues)
    pub fn with_runtime<T>(&self, mut callback: impl FnMut(&mut Runtime) -> T) -> Result<T, Error> {
        let rt_mut = self.cell_ref();
        match rt_mut.borrow_mut().as_mut() {
            Ok(rt) => Ok(callback(rt)),
            Err(e) => Err(Error::Runtime(format!(
                "Could not initialize static runtime: {e}"
            ))),
        }
    }
}

/// Create a static runtime instance
/// This macro creates a thread-local static runtime instance
///
/// The first argument is the name of the static runtime
/// The second argument is an optional block that should return a `RuntimeOptions` instance
///
/// Can be used with default `RuntimeOptions` like so:
/// ```rust
/// use rustyscript::{RuntimeOptions, Error, static_runtime};
/// use std::time::Duration;
///
/// static_runtime!(MY_DEFAULT_RUNTIME);
///
/// fn main() -> Result<(), Error> {
///     MY_DEFAULT_RUNTIME::with(|runtime| {
///         runtime.eval::<()>("console.log('Hello, world!')")
///     })
/// }
/// ```
///
/// Or with custom `RuntimeOptions`:
/// ```rust
/// use rustyscript::{Error, RuntimeOptions, static_runtime};
/// use std::time::Duration;
///
/// static_runtime!(MY_CUSTOM_RUNTIME, {
///    RuntimeOptions {
///        timeout: Duration::from_secs(5),
///        ..Default::default()
///    }
/// });
///
/// fn main() -> Result<(), Error> {
///     MY_CUSTOM_RUNTIME::with(|runtime| {
///         runtime.eval::<()>("console.log('Hello, world!')")
///     })
/// }
/// ```
#[macro_export]
macro_rules! static_runtime {
    ($name:ident, $options:block) => {
        /// A thread-local static runtime instance
        /// Use the `with` method to access the runtime
        #[allow(non_snake_case)]
        mod $name {

            fn init_options() -> $crate::RuntimeOptions {
                #[allow(unused_imports)]
                use super::*;

                $options
            }

            thread_local! {
                static RUNTIME: $crate::static_runtime::StaticRuntime
                    = const { $crate::static_runtime::StaticRuntime::new(init_options) };
            }

            /// Perform an operation on the runtime instance
            #[allow(dead_code)]
            pub fn with<T, F>(callback: F) -> Result<T, $crate::Error>
            where
                F: FnMut(&mut $crate::Runtime) -> Result<T, $crate::Error>,
            {
                RUNTIME.with(|rt| rt.with_runtime(callback))?
            }
        }
    };

    ($name:ident) => {
        static_runtime!($name, { $crate::RuntimeOptions::default() });
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;

    static_runtime!(MY_DEFAULT_RUNTIME);
    static_runtime!(MY_CUSTOM_RUNTIME, {
        RuntimeOptions {
            timeout: Duration::from_secs(5),
            ..Default::default()
        }
    });

    #[test]
    fn test_static_runtime() {
        MY_DEFAULT_RUNTIME::with(|runtime| runtime.eval::<()>("console.log('Hello, world!')"))
            .unwrap();

        MY_CUSTOM_RUNTIME::with(|runtime| runtime.eval::<()>("console.log('Hello, world!')"))
            .unwrap();
    }
}
