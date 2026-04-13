use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::cache::Cache;
use crate::db::DbPool;
use crate::error::AppError;
use crate::models::user_key::{UserKey, UserKeyCreated, UserKeyInfo};

const CACHE_ACTIVE_KEYS_SET: &str = "gateway:active_key_hashes";

/// Generate a new key in the format `sk-{uuid v4}`
pub fn generate_key() -> String {
    format!("sk-{}", Uuid::new_v4())
}

/// SHA-256 hash of a plaintext key
pub fn hash_key(plain: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(plain.as_bytes());
    hex::encode(hasher.finalize())
}

/// Extract a display prefix from a key, e.g. "sk-550e8400..." → "sk-550e..."
fn key_prefix(plain: &str) -> String {
    if plain.len() > 11 {
        format!("{}...", &plain[..11])
    } else {
        plain.to_string()
    }
}

/// Create a new user key, persist to DB + cache.
/// Returns the full key info plus the plaintext key (shown only once).
pub async fn create_key(
    name: &str,
    token_budget: Option<i64>,
    db: &DbPool,
    cache: &mut Cache,
) -> Result<UserKeyCreated, AppError> {
    let id = Uuid::new_v4();
    let plain = generate_key();
    let hash = hash_key(&plain);
    let prefix = key_prefix(&plain);
    let now = Utc::now();

    db_execute!(
        db,
        r#"
        INSERT INTO user_keys (id, name, key_hash, key_prefix, is_active, token_budget, tokens_used, created_at, updated_at)
        VALUES ($1, $2, $3, $4, TRUE, $5, 0, $6, $6)
        "#,
        id, name, &hash, &prefix, token_budget, now
    )?;

    // Add hash to active set
    cache.sadd(CACHE_ACTIVE_KEYS_SET, &hash).await?;

    Ok(UserKeyCreated {
        id,
        name: name.to_string(),
        key: plain,
        key_prefix: prefix,
        created_at: now,
    })
}

/// Result of a successful key validation.
pub struct KeyValidation {
    pub key_id: Uuid,
    pub key_hash: String,
    pub token_budget: Option<i64>,
    pub tokens_used: i64,
}

/// Validate a plaintext key against cache (fast path) or DB (slow path + backfill).
/// Returns `Some(KeyValidation)` on success, `None` on invalid key.
pub async fn validate_key(
    plain: &str,
    cache: &mut Cache,
    db: &DbPool,
) -> Result<Option<KeyValidation>, AppError> {
    let hash = hash_key(plain);

    // Fast path: check cache
    let exists = cache.sismember(CACHE_ACTIVE_KEYS_SET, &hash).await?;
    if exists {
        // Look up key details from DB
        let row: Option<(Uuid, Option<i64>, i64)> = db_query_as!(
            optional, db,
            "SELECT id, token_budget, tokens_used FROM user_keys WHERE key_hash = $1 AND is_active = TRUE",
            &hash
        )?;

        return Ok(row.map(|(id, budget, used)| KeyValidation {
            key_id: id,
            key_hash: hash,
            token_budget: budget,
            tokens_used: used,
        }));
    }

    // Slow path: check DB
    let row: Option<(Uuid, Option<i64>, i64)> = db_query_as!(
        optional, db,
        "SELECT id, token_budget, tokens_used FROM user_keys WHERE key_hash = $1 AND is_active = TRUE",
        &hash
    )?;

    if let Some((id, budget, used)) = row {
        // Backfill cache
        cache.sadd(CACHE_ACTIVE_KEYS_SET, &hash).await?;
        return Ok(Some(KeyValidation {
            key_id: id,
            key_hash: hash,
            token_budget: budget,
            tokens_used: used,
        }));
    }

    Ok(None)
}

/// List all keys (without exposing hashes or plaintext).
pub async fn list_keys(db: &DbPool) -> Result<Vec<UserKeyInfo>, AppError> {
    let keys: Vec<UserKey> = db_query_as!(
        all, db,
        "SELECT * FROM user_keys ORDER BY created_at DESC"
    )?;

    Ok(keys.into_iter().map(UserKeyInfo::from).collect())
}

/// Rotate a key: invalidate the old key and generate a new one for the same record.
/// Returns the new plaintext key (shown only once).
pub async fn rotate_key(
    id: Uuid,
    db: &DbPool,
    cache: &mut Cache,
) -> Result<UserKeyCreated, AppError> {
    // Fetch the existing key to get its old hash
    let existing: UserKey = db_query_as!(
        optional, db,
        "SELECT * FROM user_keys WHERE id = $1 AND is_active = TRUE",
        id
    )?
    .ok_or(AppError::NotFound)?;

    // Remove old hash from cache
    cache.srem(CACHE_ACTIVE_KEYS_SET, &existing.key_hash).await?;

    // Generate new key
    let new_plain = generate_key();
    let new_hash = hash_key(&new_plain);
    let new_prefix = key_prefix(&new_plain);
    let now = Utc::now();

    db_execute!(
        db,
        "UPDATE user_keys SET key_hash = $1, key_prefix = $2, updated_at = $3 WHERE id = $4",
        &new_hash, &new_prefix, now, id
    )?;

    // Add new hash to cache
    cache.sadd(CACHE_ACTIVE_KEYS_SET, &new_hash).await?;

    Ok(UserKeyCreated {
        id,
        name: existing.name,
        key: new_plain,
        key_prefix: new_prefix,
        created_at: existing.created_at,
    })
}

/// Soft-delete a key: mark inactive + remove from cache.
pub async fn delete_key(
    id: Uuid,
    db: &DbPool,
    cache: &mut Cache,
) -> Result<(), AppError> {
    let existing: UserKey = db_query_as!(
        optional, db,
        "SELECT * FROM user_keys WHERE id = $1 AND is_active = TRUE",
        id
    )?
    .ok_or(AppError::NotFound)?;

    let now = Utc::now();
    db_execute!(
        db,
        "UPDATE user_keys SET is_active = FALSE, updated_at = $1 WHERE id = $2",
        now, id
    )?;

    cache.srem(CACHE_ACTIVE_KEYS_SET, &existing.key_hash).await?;

    Ok(())
}

/// Warm up cache with all active key hashes from DB (call on startup).
pub async fn warm_up_cache(
    db: &DbPool,
    cache: &mut Cache,
) -> Result<(), AppError> {
    let hashes: Vec<String> = db_query_scalar!(
        all, db,
        "SELECT key_hash FROM user_keys WHERE is_active = TRUE"
    )?;

    if !hashes.is_empty() {
        // Clear stale data and re-populate
        cache.del(CACHE_ACTIVE_KEYS_SET).await?;

        for hash in &hashes {
            cache.sadd(CACHE_ACTIVE_KEYS_SET, hash).await?;
        }

        tracing::info!("Warmed up cache with {} active key hashes", hashes.len());
    } else {
        tracing::info!("No active keys to warm up in cache");
    }

    Ok(())
}

/// Update token budget and optionally reset usage for a key.
pub async fn update_key_budget(
    id: Uuid,
    token_budget: Option<i64>,
    reset_usage: bool,
    db: &DbPool,
) -> Result<UserKeyInfo, AppError> {
    let now = Utc::now();
    let key: Option<UserKey> = if reset_usage {
        db_query_as!(
            optional, db,
            "UPDATE user_keys SET token_budget = $1, tokens_used = 0, updated_at = $2 WHERE id = $3 RETURNING *",
            token_budget, now, id
        )?
    } else {
        db_query_as!(
            optional, db,
            "UPDATE user_keys SET token_budget = $1, updated_at = $2 WHERE id = $3 RETURNING *",
            token_budget, now, id
        )?
    };

    key.map(UserKeyInfo::from).ok_or(AppError::NotFound)
}

/// Atomically increment tokens_used for a key.
pub async fn increment_tokens_used(
    id: Uuid,
    tokens: i64,
    db: &DbPool,
) -> Result<(), AppError> {
    let now = Utc::now();
    db_execute!(
        db,
        "UPDATE user_keys SET tokens_used = tokens_used + $1, updated_at = $2 WHERE id = $3",
        tokens, now, id
    )?;
    Ok(())
}
