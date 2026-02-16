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
    token_budget: Option<i64>,
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
        INSERT INTO user_keys (id, name, key_hash, key_prefix, is_active, token_budget, tokens_used, created_at, updated_at)
        VALUES ($1, $2, $3, $4, TRUE, $5, 0, $6, $6)
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(&hash)
    .bind(&prefix)
    .bind(token_budget)
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

/// Result of a successful key validation.
pub struct KeyValidation {
    pub key_id: Uuid,
    pub key_hash: String,
    pub token_budget: Option<i64>,
    pub tokens_used: i64,
}

/// Validate a plaintext key against Redis (fast path) or PG (slow path + backfill).
/// Returns `Some(KeyValidation)` on success, `None` on invalid key.
pub async fn validate_key(
    plain: &str,
    redis: &mut ConnectionManager,
    db: &PgPool,
) -> Result<Option<KeyValidation>, AppError> {
    let hash = hash_key(plain);

    // Fast path: check Redis SET
    let exists: bool = redis.sismember(REDIS_ACTIVE_KEYS_SET, &hash).await?;
    if exists {
        // Look up key details from PG
        let row = sqlx::query_as::<_, (Uuid, Option<i64>, i64)>(
            "SELECT id, token_budget, tokens_used FROM user_keys WHERE key_hash = $1 AND is_active = TRUE",
        )
        .bind(&hash)
        .fetch_optional(db)
        .await?;

        return Ok(row.map(|(id, budget, used)| KeyValidation {
            key_id: id,
            key_hash: hash,
            token_budget: budget,
            tokens_used: used,
        }));
    }

    // Slow path: check PG
    let row = sqlx::query_as::<_, (Uuid, Option<i64>, i64)>(
        "SELECT id, token_budget, tokens_used FROM user_keys WHERE key_hash = $1 AND is_active = TRUE",
    )
    .bind(&hash)
    .fetch_optional(db)
    .await?;

    if let Some((id, budget, used)) = row {
        // Backfill Redis
        let _: () = redis.sadd(REDIS_ACTIVE_KEYS_SET, &hash).await?;
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
/// Computes weighted token usage from request_logs with model coefficients.
pub async fn list_keys(db: &PgPool) -> Result<Vec<UserKeyInfo>, AppError> {
    let keys = sqlx::query_as::<_, UserKey>("SELECT * FROM user_keys ORDER BY created_at DESC")
        .fetch_all(db)
        .await?;

    // Compute per-key weighted token usage from request_logs
    let weighted: std::collections::HashMap<Uuid, i64> = sqlx::query_as::<_, (Uuid, i64)>(
        r#"
        SELECT r.user_key_id,
               COALESCE(SUM(
                   ROUND(
                       COALESCE(r.prompt_tokens, 0) * COALESCE(m.input_token_coefficient, 1.0)
                       + COALESCE(r.completion_tokens, 0) * COALESCE(m.output_token_coefficient, 1.0)
                   )
               ), 0)::BIGINT AS weighted_total
        FROM request_logs r
        LEFT JOIN models m ON m.name = r.model_requested
        WHERE r.user_key_id IS NOT NULL
        GROUP BY r.user_key_id
        "#,
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .collect();

    Ok(keys
        .into_iter()
        .map(|k| {
            let wt = weighted.get(&k.id).copied().unwrap_or(k.tokens_used);
            let mut info = UserKeyInfo::from(k);
            info.weighted_tokens_used = wt;
            info
        })
        .collect())
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

/// Update token budget and optionally reset usage for a key.
pub async fn update_key_budget(
    id: Uuid,
    token_budget: Option<i64>,
    reset_usage: bool,
    db: &PgPool,
) -> Result<UserKeyInfo, AppError> {
    let key = if reset_usage {
        sqlx::query_as::<_, UserKey>(
            "UPDATE user_keys SET token_budget = $1, tokens_used = 0, updated_at = NOW() WHERE id = $2 RETURNING *",
        )
        .bind(token_budget)
        .bind(id)
        .fetch_optional(db)
        .await?
    } else {
        sqlx::query_as::<_, UserKey>(
            "UPDATE user_keys SET token_budget = $1, updated_at = NOW() WHERE id = $2 RETURNING *",
        )
        .bind(token_budget)
        .bind(id)
        .fetch_optional(db)
        .await?
    };

    key.map(UserKeyInfo::from).ok_or(AppError::NotFound)
}

/// Atomically increment tokens_used for a key.
pub async fn increment_tokens_used(
    id: Uuid,
    tokens: i64,
    db: &PgPool,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE user_keys SET tokens_used = tokens_used + $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(tokens)
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}
