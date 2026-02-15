use chrono::Utc;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::user_key::{UserKey, UserKeyCreated, UserKeyInfo};

const REDIS_ACTIVE_KEYS_SET: &str = "gateway:active_key_hashes";

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

/// Create a new user key, persist to PG + cache in Redis.
/// Returns the full key info plus the plaintext key (shown only once).
pub async fn create_key(
    name: &str,
    db: &PgPool,
    redis: &mut ConnectionManager,
) -> Result<UserKeyCreated, AppError> {
    let id = Uuid::new_v4();
    let plain = generate_key();
    let hash = hash_key(&plain);
    let prefix = key_prefix(&plain);
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO user_keys (id, name, key_hash, key_prefix, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, TRUE, $5, $5)
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(&hash)
    .bind(&prefix)
    .bind(now)
    .execute(db)
    .await?;

    // Add hash to Redis active set
    let _: () = redis.sadd(REDIS_ACTIVE_KEYS_SET, &hash).await?;

    Ok(UserKeyCreated {
        id,
        name: name.to_string(),
        key: plain,
        key_prefix: prefix,
        created_at: now,
    })
}

/// Validate a plaintext key against Redis (fast path) or PG (slow path + backfill).
/// Returns `Some((key_id, key_hash))` on success, `None` on invalid key.
pub async fn validate_key(
    plain: &str,
    redis: &mut ConnectionManager,
    db: &PgPool,
) -> Result<Option<(Uuid, String)>, AppError> {
    let hash = hash_key(plain);

    // Fast path: check Redis SET
    let exists: bool = redis.sismember(REDIS_ACTIVE_KEYS_SET, &hash).await?;
    if exists {
        // Look up key ID from PG (lightweight query, cached by PG)
        let key_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM user_keys WHERE key_hash = $1 AND is_active = TRUE",
        )
        .bind(&hash)
        .fetch_optional(db)
        .await?;

        return Ok(key_id.map(|id| (id, hash)));
    }

    // Slow path: check PG
    let key_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM user_keys WHERE key_hash = $1 AND is_active = TRUE",
    )
    .bind(&hash)
    .fetch_optional(db)
    .await?;

    if let Some(id) = key_id {
        // Backfill Redis
        let _: () = redis.sadd(REDIS_ACTIVE_KEYS_SET, &hash).await?;
        return Ok(Some((id, hash)));
    }

    Ok(None)
}

/// List all keys (without exposing hashes or plaintext).
pub async fn list_keys(db: &PgPool) -> Result<Vec<UserKeyInfo>, AppError> {
    let keys = sqlx::query_as::<_, UserKey>("SELECT * FROM user_keys ORDER BY created_at DESC")
        .fetch_all(db)
        .await?;

    Ok(keys.into_iter().map(UserKeyInfo::from).collect())
}

/// Rotate a key: invalidate the old key and generate a new one for the same record.
/// Returns the new plaintext key (shown only once).
pub async fn rotate_key(
    id: Uuid,
    db: &PgPool,
    redis: &mut ConnectionManager,
) -> Result<UserKeyCreated, AppError> {
    // Fetch the existing key to get its old hash
    let existing = sqlx::query_as::<_, UserKey>(
        "SELECT * FROM user_keys WHERE id = $1 AND is_active = TRUE",
    )
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)?;

    // Remove old hash from Redis
    let _: () = redis.srem(REDIS_ACTIVE_KEYS_SET, &existing.key_hash).await?;

    // Generate new key
    let new_plain = generate_key();
    let new_hash = hash_key(&new_plain);
    let new_prefix = key_prefix(&new_plain);
    let now = Utc::now();

    sqlx::query(
        "UPDATE user_keys SET key_hash = $1, key_prefix = $2, updated_at = $3 WHERE id = $4",
    )
    .bind(&new_hash)
    .bind(&new_prefix)
    .bind(now)
    .bind(id)
    .execute(db)
    .await?;

    // Add new hash to Redis
    let _: () = redis.sadd(REDIS_ACTIVE_KEYS_SET, &new_hash).await?;

    Ok(UserKeyCreated {
        id,
        name: existing.name,
        key: new_plain,
        key_prefix: new_prefix,
        created_at: existing.created_at,
    })
}

/// Soft-delete a key: mark inactive + remove from Redis.
pub async fn delete_key(
    id: Uuid,
    db: &PgPool,
    redis: &mut ConnectionManager,
) -> Result<(), AppError> {
    let existing = sqlx::query_as::<_, UserKey>(
        "SELECT * FROM user_keys WHERE id = $1 AND is_active = TRUE",
    )
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)?;

    sqlx::query("UPDATE user_keys SET is_active = FALSE, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;

    let _: () = redis.srem(REDIS_ACTIVE_KEYS_SET, &existing.key_hash).await?;

    Ok(())
}

/// Warm up Redis with all active key hashes from PG (call on startup).
pub async fn warm_up_redis(
    db: &PgPool,
    redis: &mut ConnectionManager,
) -> Result<(), AppError> {
    let hashes = sqlx::query_scalar::<_, String>(
        "SELECT key_hash FROM user_keys WHERE is_active = TRUE",
    )
    .fetch_all(db)
    .await?;

    if !hashes.is_empty() {
        // Clear stale data and re-populate
        let _: () = redis::cmd("DEL")
            .arg(REDIS_ACTIVE_KEYS_SET)
            .query_async(redis)
            .await?;

        for hash in &hashes {
            let _: () = redis.sadd(REDIS_ACTIVE_KEYS_SET, hash).await?;
        }

        tracing::info!("Warmed up Redis with {} active key hashes", hashes.len());
    } else {
        tracing::info!("No active keys to warm up in Redis");
    }

    Ok(())
}
