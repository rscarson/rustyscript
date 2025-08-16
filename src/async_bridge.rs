use std::rc::Rc;

use tokio_util::sync::CancellationToken;

use crate::Error;

/// A wrapper around the tokio runtime allowing for borrowed usage
///
/// The borrowed variant is useful for use with `tokio::main`.
#[derive(Clone)]
pub enum TokioRuntime {
    /// The runtime is borrowed and will not be dropped when this is dropped
    Borrowed(tokio::runtime::Handle),

    /// The runtime is owned and will be dropped when this is dropped
    Owned(Rc<tokio::runtime::Runtime>),
}
impl TokioRuntime {
    /// Returns a borrowed handle to the runtime
    #[must_use]
    pub fn handle(&self) -> tokio::runtime::Handle {
        match self {
            Self::Borrowed(handle) => handle.clone(),
            Self::Owned(rt) => rt.handle().clone(),
        }
    }

    /// Runs a future to completion on this Handle's associated Runtime.
    ///
    /// This runs the given future on the current thread, blocking until it is complete, and yielding its resolved result. Any tasks or timers which the future spawns internally will be executed on the runtime.
    ///
    /// When this is used on a `current_thread` runtime, only the [`Runtime::block_on`] method can drive the IO and timer drivers, but the `Handle::block_on` method cannot drive them.
    /// This means that, when using this method on a `current_thread` runtime, anything that relies on IO or timers will not work unless there is another thread currently calling [`Runtime::block_on`] on the same runtime.
    pub fn block_on<F, T>(&self, f: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        match self {
            Self::Borrowed(handle) => handle.block_on(f),
            Self::Owned(rt) => rt.block_on(f),
        }
    }
}

/// A bridge to the tokio runtime that connects the Deno and Tokio runtimes
/// Implements common patterns used throughout the codebase
pub struct AsyncBridge {
    tokio: TokioRuntime,
    timeout: std::time::Duration,
    heap_exhausted_token: CancellationToken,
}

impl AsyncBridge {
    /// Creates a new instance with the provided options.  
    /// A new tokio runtime will be created with the provided timeout.
    pub fn new(timeout: std::time::Duration) -> Result<Self, Error> {
        let tokio = Rc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .thread_keep_alive(timeout)
                .build()?,
        );

        Ok(Self::with_tokio_runtime(timeout, tokio))
    }

    /// Creates a new instance with the provided options and a pre-configured tokio runtime.
    pub fn with_tokio_runtime(
        timeout: std::time::Duration,
        tokio: Rc<tokio::runtime::Runtime>,
    ) -> Self {
        let heap_exhausted_token = CancellationToken::new();
        let tokio = TokioRuntime::Owned(tokio);
        Self {
            tokio,
            timeout,
            heap_exhausted_token,
        }
    }

    /// Creates a new instance with the provided options and a borrowed tokio runtime handle.
    pub fn with_runtime_handle(
        timeout: std::time::Duration,
        handle: tokio::runtime::Handle,
    ) -> Self {
        let heap_exhausted_token = CancellationToken::new();
        let tokio = TokioRuntime::Borrowed(handle);
        Self {
            tokio,
            timeout,
            heap_exhausted_token,
        }
    }

    /// Access the underlying tokio runtime used for blocking operations
    #[must_use]
    pub fn tokio_runtime(&self) -> TokioRuntime {
        self.tokio.clone()
    }

    /// Destroy instance, releasing all resources
    /// Then the internal tokio runtime will be returned
    #[must_use]
    pub fn into_tokio_runtime(self) -> TokioRuntime {
        self.tokio
    }

    /// Returns the timeout for the runtime
    #[must_use]
    pub fn timeout(&self) -> std::time::Duration {
        self.timeout
    }

    /// Returns the heap exhausted token for the runtime
    /// Used to detect when the runtime has run out of memory
    #[must_use]
    pub fn heap_exhausted_token(&self) -> CancellationToken {
        self.heap_exhausted_token.clone()
    }
}

pub trait AsyncBridgeExt {
    fn bridge(&self) -> &AsyncBridge;

    fn block_on<'a, Out, F, Fut>(&'a mut self, f: F) -> Result<Out, Error>
    where
        Fut: std::future::Future<Output = Result<Out, Error>>,
        F: FnOnce(&'a mut Self) -> Fut,
    {
        let timeout = self.bridge().timeout();
        let rt = self.bridge().tokio_runtime();
        let heap_exhausted_token = self.bridge().heap_exhausted_token();

        rt.block_on(async move {
            tokio::select! {
                result = tokio::time::timeout(timeout, f(self)) => result?,
                () = heap_exhausted_token.cancelled() => Err(Error::HeapExhausted),
            }
        })
    }
}
