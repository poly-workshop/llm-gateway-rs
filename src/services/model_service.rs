use chrono::Utc;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::model::{Model, ModelInfo, ModelRoute};
use crate::models::provider::Provider;

const REDIS_MODEL_ROUTES_HASH: &str = "gateway:model_routes";

/// Create a new model mapping.
pub async fn create_model(
    name: &str,
    provider_id: Uuid,
    provider_model_name: Option<&str>,
    input_token_coefficient: f64,
    output_token_coefficient: f64,
    db: &PgPool,
    redis: &mut ConnectionManager,
) -> Result<ModelInfo, AppError> {
    // Verify provider exists
    let provider = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = $1")
        .bind(provider_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::BadRequest(format!("Provider {provider_id} not found")))?;

    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO models (id, name, provider_id, provider_model_name, is_active,
                            input_token_coefficient, output_token_coefficient, created_at, updated_at)
        VALUES ($1, $2, $3, $4, TRUE, $5, $6, $7, $7)
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(provider_id)
    .bind(provider_model_name)
    .bind(input_token_coefficient)
    .bind(output_token_coefficient)
    .bind(now)
    .execute(db)
    .await?;

    // Update Redis cache
    cache_model_route(name, provider_model_name, input_token_coefficient, output_token_coefficient, &provider, redis).await?;

    Ok(ModelInfo {
        id,
        name: name.to_string(),
        provider_id,
        provider_name: Some(provider.name),
        provider_model_name: provider_model_name.map(|s| s.to_string()),
        is_active: true,
        input_token_coefficient,
        output_token_coefficient,
        created_at: now,
        updated_at: now,
    })
}

/// List all models with their provider names.
pub async fn list_models(db: &PgPool) -> Result<Vec<ModelInfo>, AppError> {
    let rows = sqlx::query_as::<_, ModelWithProvider>(
        r#"
        SELECT m.id, m.name, m.provider_id, m.provider_model_name, m.is_active,
               m.input_token_coefficient, m.output_token_coefficient,
               m.created_at, m.updated_at, p.name AS provider_name
        FROM models m
        JOIN providers p ON m.provider_id = p.id
        ORDER BY m.created_at DESC
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ModelInfo {
            id: r.id,
            name: r.name,
            provider_id: r.provider_id,
            provider_name: Some(r.provider_name),
            provider_model_name: r.provider_model_name,
            is_active: r.is_active,
            input_token_coefficient: r.input_token_coefficient,
            output_token_coefficient: r.output_token_coefficient,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect())
}

/// Delete a model and remove from Redis cache.
pub async fn delete_model(
    id: Uuid,
    db: &PgPool,
    redis: &mut ConnectionManager,
) -> Result<(), AppError> {
    let model = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound)?;

    sqlx::query("DELETE FROM models WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;

    // Remove from Redis
    let _: () = redis.hdel(REDIS_MODEL_ROUTES_HASH, &model.name).await?;

    Ok(())
}

/// Update an existing model and rebuild Redis cache.
pub async fn update_model(
    id: Uuid,
    name: Option<&str>,
    provider_id: Option<Uuid>,
    provider_model_name: Option<Option<&str>>,
    is_active: Option<bool>,
    input_token_coefficient: Option<f64>,
    output_token_coefficient: Option<f64>,
    db: &PgPool,
    redis: &mut ConnectionManager,
) -> Result<ModelInfo, AppError> {
    let existing = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound)?;

    let new_name = name.map(|s| s.to_string()).unwrap_or(existing.name.clone());
    let new_provider_id = provider_id.unwrap_or(existing.provider_id);
    let new_provider_model_name = match provider_model_name {
        Some(opt) => opt.map(|s| s.to_string()),
        None => existing.provider_model_name.clone(),
    };
    let new_is_active = is_active.unwrap_or(existing.is_active);
    let new_input_coeff = input_token_coefficient.unwrap_or(existing.input_token_coefficient);
    let new_output_coeff = output_token_coefficient.unwrap_or(existing.output_token_coefficient);

    // If provider changed, verify it exists
    if new_provider_id != existing.provider_id {
        sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = $1")
            .bind(new_provider_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::BadRequest(format!("Provider {new_provider_id} not found")))?;
    }

    sqlx::query(
        r#"
        UPDATE models
        SET name = $1, provider_id = $2, provider_model_name = $3, is_active = $4,
            input_token_coefficient = $5, output_token_coefficient = $6, updated_at = NOW()
        WHERE id = $7
        "#,
    )
    .bind(&new_name)
    .bind(new_provider_id)
    .bind(&new_provider_model_name)
    .bind(new_is_active)
    .bind(new_input_coeff)
    .bind(new_output_coeff)
    .bind(id)
    .execute(db)
    .await?;

    // Remove old name from Redis if name changed
    if new_name != existing.name {
        let _: () = redis.hdel(REDIS_MODEL_ROUTES_HASH, &existing.name).await?;
    }

    // Rebuild the full cache to keep everything consistent
    warm_up_model_routes(db, redis).await?;

    // Fetch updated row with provider name
    let row = sqlx::query_as::<_, ModelWithProvider>(
        r#"
        SELECT m.id, m.name, m.provider_id, m.provider_model_name, m.is_active,
               m.input_token_coefficient, m.output_token_coefficient,
               m.created_at, m.updated_at, p.name AS provider_name
        FROM models m
        JOIN providers p ON m.provider_id = p.id
        WHERE m.id = $1
        "#,
    )
    .bind(id)
    .fetch_one(db)
    .await?;

    Ok(ModelInfo {
        id: row.id,
        name: row.name,
        provider_id: row.provider_id,
        provider_name: Some(row.provider_name),
        provider_model_name: row.provider_model_name,
        is_active: row.is_active,
        input_token_coefficient: row.input_token_coefficient,
        output_token_coefficient: row.output_token_coefficient,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// Resolve a user-facing model name to its routing information.
/// Fast path: Redis hash lookup. Slow path: PG query + backfill Redis.
pub async fn resolve_model_route(
    model_name: &str,
    redis: &mut ConnectionManager,
    db: &PgPool,
) -> Result<Option<ModelRoute>, AppError> {
    // Fast path: check Redis
    let cached: Option<String> = redis.hget(REDIS_MODEL_ROUTES_HASH, model_name).await?;
    if let Some(json_str) = cached {
        if let Ok(route) = serde_json::from_str::<ModelRoute>(&json_str) {
            return Ok(Some(route));
        }
    }

    // Slow path: query PG
    let row = sqlx::query_as::<_, ModelWithProviderFull>(
        r#"
        SELECT m.name AS model_name, m.provider_model_name, m.provider_id,
               m.input_token_coefficient, m.output_token_coefficient,
               p.base_url, p.api_key, p.kind AS provider_kind
        FROM models m
        JOIN providers p ON m.provider_id = p.id
        WHERE m.name = $1 AND m.is_active = TRUE AND p.is_active = TRUE
        "#,
    )
    .bind(model_name)
    .fetch_optional(db)
    .await?;

    match row {
        Some(r) => {
            let route = ModelRoute {
                provider_id: r.provider_id,
                provider_model_name: r
                    .provider_model_name
                    .unwrap_or_else(|| r.model_name.clone()),
                base_url: r.base_url,
                api_key: r.api_key,
                provider_kind: r.provider_kind,
                input_token_coefficient: r.input_token_coefficient,
                output_token_coefficient: r.output_token_coefficient,
            };

            // Backfill Redis
            if let Ok(json_str) = serde_json::to_string(&route) {
                let _: Result<(), _> = redis
                    .hset(REDIS_MODEL_ROUTES_HASH, model_name, &json_str)
                    .await;
            }

            Ok(Some(route))
        }
        None => Ok(None),
    }
}

/// Warm up Redis with all active model routes (call on startup).
pub async fn warm_up_model_routes(
    db: &PgPool,
    redis: &mut ConnectionManager,
) -> Result<(), AppError> {
    let rows = sqlx::query_as::<_, ModelWithProviderFull>(
        r#"
        SELECT m.name AS model_name, m.provider_model_name, m.provider_id,
               m.input_token_coefficient, m.output_token_coefficient,
               p.base_url, p.api_key, p.kind AS provider_kind
        FROM models m
        JOIN providers p ON m.provider_id = p.id
        WHERE m.is_active = TRUE AND p.is_active = TRUE
        "#,
    )
    .fetch_all(db)
    .await?;

    // Clear stale cache
    let _: () = redis::cmd("DEL")
        .arg(REDIS_MODEL_ROUTES_HASH)
        .query_async(redis)
        .await?;

    for r in &rows {
        let route = ModelRoute {
            provider_id: r.provider_id,
            provider_model_name: r
                .provider_model_name
                .clone()
                .unwrap_or_else(|| r.model_name.clone()),
            base_url: r.base_url.clone(),
            api_key: r.api_key.clone(),
            provider_kind: r.provider_kind.clone(),
            input_token_coefficient: r.input_token_coefficient,
            output_token_coefficient: r.output_token_coefficient,
        };

        if let Ok(json_str) = serde_json::to_string(&route) {
            let _: Result<(), _> = redis
                .hset(REDIS_MODEL_ROUTES_HASH, &r.model_name, &json_str)
                .await;
        }
    }

    tracing::info!("Warmed up Redis with {} model routes", rows.len());
    Ok(())
}

// ── Internal query types ──────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct ModelWithProvider {
    id: Uuid,
    name: String,
    provider_id: Uuid,
    provider_model_name: Option<String>,
    is_active: bool,
    input_token_coefficient: f64,
    output_token_coefficient: f64,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    provider_name: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ModelWithProviderFull {
    model_name: String,
    provider_model_name: Option<String>,
    provider_id: Uuid,
    input_token_coefficient: f64,
    output_token_coefficient: f64,
    base_url: String,
    api_key: String,
    provider_kind: String,
}

/// Cache a single model route into Redis.
async fn cache_model_route(
    model_name: &str,
    provider_model_name: Option<&str>,
    input_token_coefficient: f64,
    output_token_coefficient: f64,
    provider: &Provider,
    redis: &mut ConnectionManager,
) -> Result<(), AppError> {
    let route = ModelRoute {
        provider_id: provider.id,
        provider_model_name: provider_model_name
            .unwrap_or(model_name)
            .to_string(),
        base_url: provider.base_url.clone(),
        api_key: provider.api_key.clone(),
        provider_kind: provider.kind.clone(),
        input_token_coefficient,
        output_token_coefficient,
    };

    let json_str = serde_json::to_string(&route)
        .map_err(|e| AppError::Internal(format!("JSON serialization error: {e}")))?;

    let _: () = redis.hset(REDIS_MODEL_ROUTES_HASH, model_name, &json_str).await?;
    Ok(())
}
