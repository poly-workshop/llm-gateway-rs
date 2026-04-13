use chrono::Utc;
use uuid::Uuid;

use crate::cache::Cache;
use crate::db::DbPool;
use crate::error::AppError;
use crate::models::model::{Model, ModelInfo, ModelRoute};
use crate::models::provider::Provider;

const CACHE_MODEL_ROUTES_HASH: &str = "gateway:model_routes";

/// Create a new model mapping.
pub async fn create_model(
    name: &str,
    provider_id: Uuid,
    provider_model_name: Option<&str>,
    input_token_coefficient: f64,
    output_token_coefficient: f64,
    db: &DbPool,
    cache: &mut Cache,
) -> Result<ModelInfo, AppError> {
    // Verify provider exists
    let provider: Provider = db_query_as!(
        optional, db,
        "SELECT * FROM providers WHERE id = $1",
        provider_id
    )?
    .ok_or_else(|| AppError::BadRequest(format!("Provider {provider_id} not found")))?;

    let id = Uuid::new_v4();
    let now = Utc::now();

    db_execute!(
        db,
        r#"
        INSERT INTO models (id, name, provider_id, provider_model_name, is_active,
                            input_token_coefficient, output_token_coefficient, created_at, updated_at)
        VALUES ($1, $2, $3, $4, TRUE, $5, $6, $7, $7)
        "#,
        id, name, provider_id, provider_model_name,
        input_token_coefficient, output_token_coefficient, now
    )?;

    // Update cache
    cache_model_route(name, provider_model_name, input_token_coefficient, output_token_coefficient, &provider, cache).await?;

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
pub async fn list_models(db: &DbPool) -> Result<Vec<ModelInfo>, AppError> {
    let rows: Vec<ModelWithProvider> = db_query_as!(
        all, db,
        r#"
        SELECT m.id, m.name, m.provider_id, m.provider_model_name, m.is_active,
               m.input_token_coefficient, m.output_token_coefficient,
               m.created_at, m.updated_at, p.name AS provider_name
        FROM models m
        JOIN providers p ON m.provider_id = p.id
        ORDER BY m.created_at DESC
        "#
    )?;

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

/// Delete a model and remove from cache.
pub async fn delete_model(
    id: Uuid,
    db: &DbPool,
    cache: &mut Cache,
) -> Result<(), AppError> {
    let model: Model = db_query_as!(
        optional, db,
        "SELECT * FROM models WHERE id = $1",
        id
    )?
    .ok_or(AppError::NotFound)?;

    db_execute!(db, "DELETE FROM models WHERE id = $1", id)?;

    // Remove from cache
    cache.hdel(CACHE_MODEL_ROUTES_HASH, &model.name).await?;

    Ok(())
}

/// Update an existing model and rebuild cache.
pub async fn update_model(
    id: Uuid,
    name: Option<&str>,
    provider_id: Option<Uuid>,
    provider_model_name: Option<Option<&str>>,
    is_active: Option<bool>,
    input_token_coefficient: Option<f64>,
    output_token_coefficient: Option<f64>,
    db: &DbPool,
    cache: &mut Cache,
) -> Result<ModelInfo, AppError> {
    let existing: Model = db_query_as!(
        optional, db,
        "SELECT * FROM models WHERE id = $1",
        id
    )?
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
        let _: Provider = db_query_as!(
            optional, db,
            "SELECT * FROM providers WHERE id = $1",
            new_provider_id
        )?
        .ok_or_else(|| AppError::BadRequest(format!("Provider {new_provider_id} not found")))?;
    }

    let now = Utc::now();
    db_execute!(
        db,
        r#"
        UPDATE models
        SET name = $1, provider_id = $2, provider_model_name = $3, is_active = $4,
            input_token_coefficient = $5, output_token_coefficient = $6, updated_at = $7
        WHERE id = $8
        "#,
        &new_name, new_provider_id, &new_provider_model_name, new_is_active,
        new_input_coeff, new_output_coeff, now, id
    )?;

    // Remove old name from cache if name changed
    if new_name != existing.name {
        cache.hdel(CACHE_MODEL_ROUTES_HASH, &existing.name).await?;
    }

    // Rebuild the full cache to keep everything consistent
    warm_up_model_routes(db, cache).await?;

    // Fetch updated row with provider name
    let row: ModelWithProvider = db_query_as!(
        one, db,
        r#"
        SELECT m.id, m.name, m.provider_id, m.provider_model_name, m.is_active,
               m.input_token_coefficient, m.output_token_coefficient,
               m.created_at, m.updated_at, p.name AS provider_name
        FROM models m
        JOIN providers p ON m.provider_id = p.id
        WHERE m.id = $1
        "#,
        id
    )?;

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
/// Fast path: cache lookup. Slow path: DB query + backfill cache.
pub async fn resolve_model_route(
    model_name: &str,
    cache: &mut Cache,
    db: &DbPool,
) -> Result<Option<ModelRoute>, AppError> {
    // Fast path: check cache
    let cached: Option<String> = cache.hget(CACHE_MODEL_ROUTES_HASH, model_name).await?;
    if let Some(json_str) = cached {
        if let Ok(route) = serde_json::from_str::<ModelRoute>(&json_str) {
            return Ok(Some(route));
        }
    }

    // Slow path: query DB
    let row: Option<ModelWithProviderFull> = db_query_as!(
        optional, db,
        r#"
        SELECT m.name AS model_name, m.provider_model_name, m.provider_id,
               m.input_token_coefficient, m.output_token_coefficient,
               p.base_url, p.api_key, p.kind AS provider_kind
        FROM models m
        JOIN providers p ON m.provider_id = p.id
        WHERE m.name = $1 AND m.is_active = TRUE AND p.is_active = TRUE
        "#,
        model_name
    )?;

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

            // Backfill cache
            if let Ok(json_str) = serde_json::to_string(&route) {
                let _ = cache.hset(CACHE_MODEL_ROUTES_HASH, model_name, &json_str).await;
            }

            Ok(Some(route))
        }
        None => Ok(None),
    }
}

/// Warm up cache with all active model routes (call on startup).
pub async fn warm_up_model_routes(
    db: &DbPool,
    cache: &mut Cache,
) -> Result<(), AppError> {
    let rows: Vec<ModelWithProviderFull> = db_query_as!(
        all, db,
        r#"
        SELECT m.name AS model_name, m.provider_model_name, m.provider_id,
               m.input_token_coefficient, m.output_token_coefficient,
               p.base_url, p.api_key, p.kind AS provider_kind
        FROM models m
        JOIN providers p ON m.provider_id = p.id
        WHERE m.is_active = TRUE AND p.is_active = TRUE
        "#
    )?;

    // Clear stale cache
    cache.del(CACHE_MODEL_ROUTES_HASH).await?;

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
            let _ = cache.hset(CACHE_MODEL_ROUTES_HASH, &r.model_name, &json_str).await;
        }
    }

    tracing::info!("Warmed up cache with {} model routes", rows.len());
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

/// Cache a single model route.
async fn cache_model_route(
    model_name: &str,
    provider_model_name: Option<&str>,
    input_token_coefficient: f64,
    output_token_coefficient: f64,
    provider: &Provider,
    cache: &mut Cache,
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

    cache.hset(CACHE_MODEL_ROUTES_HASH, model_name, &json_str).await?;
    Ok(())
}
