#![allow(clippy::type_complexity)]
#![allow(clippy::type_repetition_in_bounds)]
use super::memory::{InMemoryCache, MyResource};
use deno_cache::{
    Cache, CacheDeleteRequest, CacheError, CacheMatchRequest, CacheMatchResponseMeta,
    CachePutRequest, SqliteBackedCache,
};
use deno_core::Resource;
use std::{path::Path, rc::Rc};

type SqliteMeta = <SqliteBackedCache as Cache>::CacheMatchResourceType;
pub enum ResourceType {
    Sqlite(Rc<SqliteMeta>),
    Memory(Rc<MyResource>),
}
impl Resource for ResourceType {
    fn name(&self) -> std::borrow::Cow<str> {
        match self {
            Self::Sqlite(resource) => resource.name(),
            Self::Memory(resource) => resource.name(),
        }
    }

    fn read(self: std::rc::Rc<Self>, limit: usize) -> deno_core::AsyncResult<deno_core::BufView> {
        match self.as_ref() {
            Self::Sqlite(resource) => <SqliteMeta as Resource>::read(resource.clone(), limit),
            Self::Memory(resource) => <MyResource as Resource>::read(resource.clone(), limit),
        }
    }

    fn write(
        self: std::rc::Rc<Self>,
        buf: deno_core::BufView,
    ) -> deno_core::AsyncResult<deno_core::WriteOutcome> {
        match self.as_ref() {
            Self::Sqlite(resource) => <SqliteMeta as Resource>::write(resource.clone(), buf),
            Self::Memory(resource) => <MyResource as Resource>::write(resource.clone(), buf),
        }
    }

    fn read_byob(
        self: std::rc::Rc<Self>,
        buf: deno_core::BufMutView,
    ) -> deno_core::AsyncResult<(usize, deno_core::BufMutView)> {
        match self.as_ref() {
            Self::Sqlite(resource) => <SqliteMeta as Resource>::read_byob(resource.clone(), buf),
            Self::Memory(resource) => <MyResource as Resource>::read_byob(resource.clone(), buf),
        }
    }

    fn write_sync(self: std::rc::Rc<Self>, data: &[u8]) -> Result<usize, deno_core::anyhow::Error> {
        match self.as_ref() {
            Self::Sqlite(resource) => <SqliteMeta as Resource>::write_sync(resource.clone(), data),
            Self::Memory(resource) => <MyResource as Resource>::write_sync(resource.clone(), data),
        }
    }
}

/// A cache backend that can store data in memory or an sqlite database
#[derive(Clone)]
pub enum CacheBackend {
    /// Persistent cache backend that stores data in a sqlite database
    Sqlite(deno_cache::SqliteBackedCache),

    /// Cache backend that stores data in memory
    Memory(super::memory::InMemoryCache),
}
impl Cache for CacheBackend {
    type CacheMatchResourceType = ResourceType;

    #[must_use]
    fn storage_open<'life0, 'async_trait>(
        &'life0 self,
        cache_name: String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<i64, CacheError>> + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        match self {
            Self::Sqlite(cache) => cache.storage_open(cache_name),
            Self::Memory(cache) => cache.storage_open(cache_name),
        }
    }

    #[must_use]
    fn storage_has<'life0, 'async_trait>(
        &'life0 self,
        cache_name: String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<bool, CacheError>> + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        match self {
            Self::Sqlite(cache) => cache.storage_has(cache_name),
            Self::Memory(cache) => cache.storage_has(cache_name),
        }
    }

    #[must_use]
    fn storage_delete<'life0, 'async_trait>(
        &'life0 self,
        cache_name: String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<bool, CacheError>> + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        match self {
            Self::Sqlite(cache) => cache.storage_delete(cache_name),
            Self::Memory(cache) => cache.storage_delete(cache_name),
        }
    }

    #[doc = " Put a resource into the cache."]
    #[must_use]
    fn put<'life0, 'async_trait>(
        &'life0 self,
        request_response: CachePutRequest,
        resource: Option<Rc<dyn Resource>>,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<(), CacheError>> + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        match self {
            Self::Sqlite(cache) => cache.put(request_response, resource),
            Self::Memory(cache) => cache.put(request_response, resource),
        }
    }

    #[must_use]
    fn r#match<'life0, 'async_trait>(
        &'life0 self,
        request: CacheMatchRequest,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<
                    Output = Result<
                        Option<(CacheMatchResponseMeta, Option<Self::CacheMatchResourceType>)>,
                        CacheError,
                    >,
                > + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        match self {
            Self::Sqlite(cache) => Box::pin(async move {
                let result = cache.r#match(request).await?;
                Ok(result.map(|(meta, resource)| {
                    (
                        meta,
                        resource.map(|resource| ResourceType::Sqlite(Rc::new(resource))),
                    )
                }))
            }),

            Self::Memory(cache) => Box::pin(async move {
                let result = cache.r#match(request).await?;
                Ok(result.map(|(meta, resource)| {
                    (
                        meta,
                        resource.map(|resource| ResourceType::Memory(Rc::new(resource))),
                    )
                }))
            }),
        }
    }

    #[must_use]
    fn delete<'life0, 'async_trait>(
        &'life0 self,
        request: CacheDeleteRequest,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<bool, CacheError>> + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        match self {
            Self::Sqlite(cache) => cache.delete(request),
            Self::Memory(cache) => cache.delete(request),
        }
    }
}

impl Default for CacheBackend {
    fn default() -> Self {
        Self::new_memory()
    }
}

impl CacheBackend {
    /// Create a persistent cache backend that stores data in a sqlite database
    ///
    /// # Arguments
    /// dir - The directory to store the sqlite database in
    ///
    /// # Errors
    /// Will return an error if the sqlite database cannot be created
    pub fn new_sqlite(dir: impl AsRef<Path>) -> Result<Self, CacheError> {
        let inner = deno_cache::SqliteBackedCache::new(dir.as_ref().to_path_buf())?;
        Ok(Self::Sqlite(inner))
    }

    /// Create a new cache backend that stores data in memory
    #[must_use]
    pub fn new_memory() -> Self {
        Self::Memory(InMemoryCache::new())
    }
}
