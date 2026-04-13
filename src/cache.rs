use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache abstraction: either Redis (for multi-instance deployments) or
/// an in-memory store (for single-instance / SQLite mode).
#[derive(Clone)]
pub enum Cache {
    Redis(Box<redis::aio::ConnectionManager>),
    InMemory(Arc<InMemoryCache>),
}

/// Simple in-memory cache that mirrors the Redis SET / HASH operations
/// used by key_service and model_service.
pub struct InMemoryCache {
    sets: RwLock<HashMap<String, HashSet<String>>>,
    hashes: RwLock<HashMap<String, HashMap<String, String>>>,
}

impl InMemoryCache {
    pub fn new() -> Self {
        Self {
            sets: RwLock::new(HashMap::new()),
            hashes: RwLock::new(HashMap::new()),
        }
    }
}

impl Cache {
    /// Create a new in-memory cache.
    pub fn in_memory() -> Self {
        Cache::InMemory(Arc::new(InMemoryCache::new()))
    }

    // ── SET operations ────────────────────────────────────────────────

    /// Add a member to a set.
    pub async fn sadd(&mut self, key: &str, member: &str) -> Result<(), crate::error::AppError> {
        match self {
            Cache::Redis(cm) => {
                let _: () = redis::AsyncCommands::sadd(cm.as_mut(), key, member).await?;
                Ok(())
            }
            Cache::InMemory(store) => {
                let mut sets = store.sets.write().await;
                sets.entry(key.to_string())
                    .or_default()
                    .insert(member.to_string());
                Ok(())
            }
        }
    }

    /// Check if a member exists in a set.
    pub async fn sismember(&mut self, key: &str, member: &str) -> Result<bool, crate::error::AppError> {
        match self {
            Cache::Redis(cm) => {
                let exists: bool = redis::AsyncCommands::sismember(cm.as_mut(), key, member).await?;
                Ok(exists)
            }
            Cache::InMemory(store) => {
                let sets = store.sets.read().await;
                Ok(sets.get(key).is_some_and(|s| s.contains(member)))
            }
        }
    }

    /// Remove a member from a set.
    pub async fn srem(&mut self, key: &str, member: &str) -> Result<(), crate::error::AppError> {
        match self {
            Cache::Redis(cm) => {
                let _: () = redis::AsyncCommands::srem(cm.as_mut(), key, member).await?;
                Ok(())
            }
            Cache::InMemory(store) => {
                let mut sets = store.sets.write().await;
                if let Some(set) = sets.get_mut(key) {
                    set.remove(member);
                }
                Ok(())
            }
        }
    }

    // ── HASH operations ───────────────────────────────────────────────

    /// Get a field from a hash.
    pub async fn hget(&mut self, key: &str, field: &str) -> Result<Option<String>, crate::error::AppError> {
        match self {
            Cache::Redis(cm) => {
                let val: Option<String> = redis::AsyncCommands::hget(cm.as_mut(), key, field).await?;
                Ok(val)
            }
            Cache::InMemory(store) => {
                let hashes = store.hashes.read().await;
                Ok(hashes
                    .get(key)
                    .and_then(|h| h.get(field))
                    .cloned())
            }
        }
    }

    /// Set a field in a hash.
    pub async fn hset(&mut self, key: &str, field: &str, value: &str) -> Result<(), crate::error::AppError> {
        match self {
            Cache::Redis(cm) => {
                let _: () = redis::AsyncCommands::hset(cm.as_mut(), key, field, value).await?;
                Ok(())
            }
            Cache::InMemory(store) => {
                let mut hashes = store.hashes.write().await;
                hashes
                    .entry(key.to_string())
                    .or_default()
                    .insert(field.to_string(), value.to_string());
                Ok(())
            }
        }
    }

    /// Remove a field from a hash.
    pub async fn hdel(&mut self, key: &str, field: &str) -> Result<(), crate::error::AppError> {
        match self {
            Cache::Redis(cm) => {
                let _: () = redis::AsyncCommands::hdel(cm.as_mut(), key, field).await?;
                Ok(())
            }
            Cache::InMemory(store) => {
                let mut hashes = store.hashes.write().await;
                if let Some(hash) = hashes.get_mut(key) {
                    hash.remove(field);
                }
                Ok(())
            }
        }
    }

    // ── Key-level operations ──────────────────────────────────────────

    /// Delete an entire key (set or hash).
    pub async fn del(&mut self, key: &str) -> Result<(), crate::error::AppError> {
        match self {
            Cache::Redis(cm) => {
                let _: () = redis::cmd("DEL")
                    .arg(key)
                    .query_async(cm.as_mut())
                    .await?;
                Ok(())
            }
            Cache::InMemory(store) => {
                {
                    let mut sets = store.sets.write().await;
                    sets.remove(key);
                }
                {
                    let mut hashes = store.hashes.write().await;
                    hashes.remove(key);
                }
                Ok(())
            }
        }
    }
}
