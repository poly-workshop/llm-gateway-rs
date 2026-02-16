use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::AppError;
use crate::services::{key_service, log_service, model_service, provider_service};
use crate::state::AppState;

// ── User Key endpoints ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
    pub token_budget: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateKeyRequest {
    /// Token budget. null = unlimited.
    pub token_budget: Option<i64>,
    /// If true, reset tokens_used to 0.
    #[serde(default)]
    pub reset_usage: bool,
}

/// POST /admin/keys — create a new user key
async fn create_key(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateKeyRequest>,
) -> Result<impl IntoResponse, AppError> {
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }

    let mut redis = state.redis.clone();
    let result = key_service::create_key(&body.name, body.token_budget, &state.db, &mut redis).await?;

    Ok((StatusCode::CREATED, Json(result)))
}

/// GET /admin/keys — list all keys (without plaintext)
async fn list_keys(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<crate::models::user_key::UserKeyInfo>>, AppError> {
    let keys = key_service::list_keys(&state.db).await?;
    Ok(Json(keys))
}

/// POST /admin/keys/:id/rotate — rotate a key, return new plaintext
async fn rotate_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::models::user_key::UserKeyCreated>, AppError> {
    let mut redis = state.redis.clone();
    let result = key_service::rotate_key(id, &state.db, &mut redis).await?;
    Ok(Json(result))
}

/// DELETE /admin/keys/:id — soft-delete a key
async fn delete_key_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let mut redis = state.redis.clone();
    key_service::delete_key(id, &state.db, &mut redis).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /admin/keys/:id — update key budget / reset usage
async fn update_key_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateKeyRequest>,
) -> Result<Json<crate::models::user_key::UserKeyInfo>, AppError> {
    let result = key_service::update_key_budget(
        id,
        body.token_budget,
        body.reset_usage,
        &state.db,
    )
    .await?;
    Ok(Json(result))
}

// ── Provider endpoints ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    /// "openai" | "openrouter" | "dashscope"
    pub kind: String,
    /// Optional; defaults based on kind
    pub base_url: Option<String>,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub kind: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub is_active: Option<bool>,
}

/// POST /admin/providers
async fn create_provider(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateProviderRequest>,
) -> Result<impl IntoResponse, AppError> {
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }

    let result = provider_service::create_provider(
        &body.name,
        &body.kind,
        body.base_url.as_deref(),
        &body.api_key,
        &state.db,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(result)))
}

/// GET /admin/providers
async fn list_providers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<crate::models::provider::ProviderInfo>>, AppError> {
    let providers = provider_service::list_providers(&state.db).await?;
    Ok(Json(providers))
}

/// PUT /admin/providers/:id
async fn update_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProviderRequest>,
) -> Result<Json<crate::models::provider::ProviderInfo>, AppError> {
    let result = provider_service::update_provider(
        id,
        body.name.as_deref(),
        body.kind.as_deref(),
        body.base_url.as_deref(),
        body.api_key.as_deref(),
        body.is_active,
        &state.db,
    )
    .await?;

    // Rebuild model route cache since provider details may have changed
    let mut redis = state.redis.clone();
    model_service::warm_up_model_routes(&state.db, &mut redis).await?;

    Ok(Json(result))
}

/// DELETE /admin/providers/:id
async fn delete_provider_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    provider_service::delete_provider(id, &state.db).await?;

    // Rebuild model route cache
    let mut redis = state.redis.clone();
    model_service::warm_up_model_routes(&state.db, &mut redis).await?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Model endpoints ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateModelRequest {
    /// User-facing model name (e.g. "gpt-4o")
    pub name: String,
    /// Provider UUID
    pub provider_id: Uuid,
    /// Optional: actual model name on the provider side (defaults to `name`)
    pub provider_model_name: Option<String>,
    /// Token budget coefficient for prompt tokens (default 1.0)
    pub input_token_coefficient: Option<f64>,
    /// Token budget coefficient for completion tokens (default 1.0)
    pub output_token_coefficient: Option<f64>,
}

/// POST /admin/models
async fn create_model(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateModelRequest>,
) -> Result<impl IntoResponse, AppError> {
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }

    let mut redis = state.redis.clone();
    let result = model_service::create_model(
        &body.name,
        body.provider_id,
        body.provider_model_name.as_deref(),
        body.input_token_coefficient.unwrap_or(1.0),
        body.output_token_coefficient.unwrap_or(1.0),
        &state.db,
        &mut redis,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(result)))
}

/// GET /admin/models
async fn list_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<crate::models::model::ModelInfo>>, AppError> {
    let models = model_service::list_models(&state.db).await?;
    Ok(Json(models))
}

/// DELETE /admin/models/:id
async fn delete_model_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let mut redis = state.redis.clone();
    model_service::delete_model(id, &state.db, &mut redis).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelRequest {
    pub name: Option<String>,
    pub provider_id: Option<Uuid>,
    /// Use `null` to reset to default (= model name). Omit the field to keep current value.
    pub provider_model_name: Option<Option<String>>,
    pub is_active: Option<bool>,
    pub input_token_coefficient: Option<f64>,
    pub output_token_coefficient: Option<f64>,
}

/// PUT /admin/models/:id
async fn update_model_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateModelRequest>,
) -> Result<Json<crate::models::model::ModelInfo>, AppError> {
    let mut redis = state.redis.clone();
    let result = model_service::update_model(
        id,
        body.name.as_deref(),
        body.provider_id,
        body.provider_model_name.as_ref().map(|o| o.as_deref()),
        body.is_active,
        body.input_token_coefficient,
        body.output_token_coefficient,
        &state.db,
        &mut redis,
    )
    .await?;

    Ok(Json(result))
}

// ── Router ────────────────────────────────────────────────────────────

// ── Request Log endpoints ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListLogsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub key_id: Option<Uuid>,
    pub model: Option<String>,
}

/// GET /admin/logs — list request logs with pagination + optional filters
async fn list_logs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListLogsQuery>,
) -> Result<Json<crate::models::request_log::LogListResponse>, AppError> {
    let params = log_service::ListLogsParams {
        page: query.page.unwrap_or(1).max(1),
        per_page: query.per_page.unwrap_or(50).min(200).max(1),
        key_id: query.key_id,
        model: query.model,
    };
    let result = log_service::list_logs(&state.db, params).await?;
    Ok(Json(result))
}

/// Build the admin router (to be nested under /admin)
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // User keys
        .route("/keys", post(create_key).get(list_keys))
        .route("/keys/{id}", delete(delete_key_handler).put(update_key_handler))
        .route("/keys/{id}/rotate", post(rotate_key))
        // Providers
        .route("/providers", post(create_provider).get(list_providers))
        .route("/providers/{id}", delete(delete_provider_handler).put(update_provider))
        // Models
        .route("/models", post(create_model).get(list_models))
        .route("/models/{id}", delete(delete_model_handler).put(update_model_handler))
        // Logs
        .route("/logs", get(list_logs))
}
