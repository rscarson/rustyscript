use crate::Error;
use std::rc::Rc;
use tokio_util::sync::CancellationToken;

/// A bridge to the tokio runtime that connects the Deno and Tokio runtimes
/// Implements common patterns used throughout the codebase
pub struct AsyncBridge {
    tokio: Rc<tokio::runtime::Runtime>,
    timeout: std::time::Duration,
    heap_exhausted_token: CancellationToken,
}

impl AsyncBridge {
    /// Creates a new instance with the provided options.
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
        Self {
            tokio,
            timeout,
            heap_exhausted_token,
        }
    }

    /// Access the underlying tokio runtime used for blocking operations
    #[must_use]
    pub fn tokio_runtime(&self) -> std::rc::Rc<tokio::runtime::Runtime> {
        self.tokio.clone()
    }

    /// Destroy instance, releasing all resources
    /// Then the internal tokio runtime will be returned
    #[must_use]
    pub fn into_tokio_runtime(self) -> Rc<tokio::runtime::Runtime> {
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
