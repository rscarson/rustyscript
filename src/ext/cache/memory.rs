#![allow(clippy::type_complexity)]
#![allow(clippy::type_repetition_in_bounds)]
use deno_cache::{
    Cache, CacheDeleteRequest, CacheError, CacheMatchRequest, CacheMatchResponseMeta,
    CachePutRequest,
};
use deno_core::{
    anyhow::{anyhow, Error},
    AsyncResult, BufView, ByteString, Resource, ResourceId,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, PartialEq)]
struct Request {
    pub url: String,
    pub headers: Vec<(ByteString, ByteString)>,
}

#[derive(Clone)]
#[allow(unused)]
struct Response {
    pub headers: Vec<(ByteString, ByteString)>,
    pub status: u16,
    pub status_text: String,
    pub rid: Option<ResourceId>,
    pub body: Option<MyResource>,
}

#[derive(Clone)]
pub struct MyResource(Rc<dyn Resource>);
impl Resource for MyResource {
    fn read(self: Rc<Self>, limit: usize) -> AsyncResult<BufView> {
        self.0.clone().read(limit)
    }

    fn read_byob_sync(self: Rc<Self>, data: &mut [u8]) -> Result<usize, Error> {
        self.0.clone().read_byob_sync(data)
    }

    fn read_byob(
        self: Rc<Self>,
        buf: deno_core::BufMutView,
    ) -> AsyncResult<(usize, deno_core::BufMutView)> {
        self.0.clone().read_byob(buf)
    }

    fn write(self: Rc<Self>, buf: BufView) -> AsyncResult<deno_core::WriteOutcome> {
        self.0.clone().write(buf)
    }

    fn write_sync(self: Rc<Self>, data: &[u8]) -> Result<usize, Error> {
        self.0.clone().write_sync(data)
    }
}

#[derive(Clone)]
struct CacheEntry {
    pub id: i64,
    pub name: String,
    entries: Vec<(Request, Response)>,
}

#[derive(Clone)]
pub struct InnerInMemoryCache {
    next_id: i64,
    entries: Vec<CacheEntry>,
}

impl Default for InnerInMemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl InnerInMemoryCache {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            entries: Vec::new(),
        }
    }

    pub fn insert(&mut self, name: String) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(CacheEntry {
            id,
            name,
            entries: Vec::new(),
        });
        id
    }
}

#[derive(Clone)]
pub struct InMemoryCache {
    inner: Rc<RefCell<InnerInMemoryCache>>,
}
impl InMemoryCache {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(InnerInMemoryCache::new())),
        }
    }
}
impl Default for InMemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache for InMemoryCache {
    type CacheMatchResourceType = MyResource;

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
        Box::pin(async move {
            let inner = self.inner.clone();
            let mut inner = inner.borrow_mut();

            let cache = inner.entries.iter().find(|entry| entry.name == cache_name);
            if let Some(cache) = cache {
                Ok(cache.id)
            } else {
                Ok(inner.insert(cache_name))
            }
        })
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
        Box::pin(async move {
            let inner = self.inner.clone();
            let inner = inner.borrow();

            let cache = inner.entries.iter().find(|entry| entry.name == cache_name);
            Ok(cache.is_some())
        })
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
        Box::pin(async move {
            let inner = self.inner.clone();
            let mut inner = inner.borrow_mut();

            let cache = inner.entries.iter().find(|entry| entry.name == cache_name);
            let id = cache.map(|entry| entry.id);
            if let Some(id) = id {
                inner.entries.retain(|entry| entry.id != id);
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }

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
        Box::pin(async move {
            let inner = self.inner.clone();
            let mut inner = inner.borrow_mut();

            let cache = inner
                .entries
                .iter_mut()
                .find(|entry| entry.id == request_response.cache_id);

            let request = Request {
                url: request_response.request_url,
                headers: request_response.request_headers,
            };

            let response = Response {
                headers: request_response.response_headers,
                status: request_response.response_status,
                status_text: request_response.response_status_text,
                rid: request_response.response_rid,
                body: resource.map(MyResource),
            };

            if let Some(cache) = cache {
                cache.entries.retain(|(req, _)| req != &request);
                cache.entries.push((request, response));
            } else {
                return Err(CacheError::Resource(anyhow!("Cache not found")));
            }

            Ok(())
        })
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
        Box::pin(async move {
            let inner = self.inner.clone();
            let inner = inner.borrow();

            let cache = inner
                .entries
                .iter()
                .find(|entry| entry.id == request.cache_id);

            if let Some(cache) = cache {
                let entry = cache
                    .entries
                    .iter()
                    .find(|(req, _)| req.url == request.request_url);

                if let Some((_, response)) = entry {
                    let response = response.clone();
                    let body = response.body;
                    let response = CacheMatchResponseMeta {
                        response_headers: response.headers,
                        response_status: response.status,
                        response_status_text: response.status_text,
                        request_headers: request.request_headers,
                    };

                    Ok(Some((response, body)))
                } else {
                    Ok(None)
                }
            } else {
                Err(CacheError::Resource(anyhow!("Cache not found")))
            }
        })
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
        Box::pin(async move {
            let inner = self.inner.clone();
            let mut inner = inner.borrow_mut();

            let cache = inner
                .entries
                .iter_mut()
                .find(|entry| entry.id == request.cache_id);
            if let Some(cache) = cache {
                let matches = cache
                    .entries
                    .iter()
                    .filter(|(req, _)| req.url == request.request_url)
                    .count();
                cache
                    .entries
                    .retain(|(req, _)| req.url != request.request_url);
                Ok(matches > 0)
            } else {
                Err(CacheError::Resource(anyhow!("Cache not found")))
            }
        })
    }
}
