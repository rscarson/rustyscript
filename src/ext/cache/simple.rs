use deno_cache::{CreateCache, CacheError};
use std::{path::Path, sync::Arc};

/// Create a cache backed by SQLite
pub fn sqlite_cache(dir: impl AsRef<Path>) -> Result<CreateCache, CacheError> {
    let dir = dir.as_ref().to_path_buf();
    let f = move || {
        let inner = deno_cache::SqliteBackedCache::new(dir.clone())?;
        Ok(deno_cache::CacheImpl::Sqlite(inner))
    };
    Ok(CreateCache(Arc::new(f)))
}

/// Create a temporary cache for testing
pub fn temp_cache() -> CreateCache {
    let f = || {
        // Use a temporary directory for in-memory-like behavior
        let temp_dir = std::env::temp_dir().join(format!("rustyscript_cache_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).map_err(|e| CacheError::Io(e))?;
        let inner = deno_cache::SqliteBackedCache::new(temp_dir)?;
        Ok(deno_cache::CacheImpl::Sqlite(inner))
    };
    CreateCache(Arc::new(f))
}