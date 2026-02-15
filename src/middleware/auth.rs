use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::services::key_service;
use crate::state::AppState;

/// Identity of the authenticated user key, injected into request extensions.
#[derive(Debug, Clone)]
pub struct KeyIdentity {
    pub key_id: Uuid,
    pub key_hash: String,
    pub token_budget: Option<i64>,
    pub tokens_used: i64,
}

/// Extract a Bearer token from the Authorization header.
fn extract_bearer(req: &Request) -> Option<&str> {
    req.headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

/// Middleware that validates the Admin Key from env config.
pub async fn admin_auth(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    let token = match extract_bearer(&req) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": { "message": "Missing Authorization header" } })),
            )
                .into_response()
        }
    };

    if token != state.config.admin_key {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": { "message": "Invalid admin key" } })),
        )
            .into_response();
    }

    next.run(req).await
}

/// Middleware that validates a User Key against Redis / PG.
pub async fn user_key_auth(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    let token = match extract_bearer(&req) {
        Some(t) => t.to_string(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": { "message": "Missing Authorization header" } })),
            )
                .into_response()
        }
    };

    let mut redis = state.redis.clone();
    match key_service::validate_key(&token, &mut redis, &state.db).await {
        Ok(Some(v)) => {
            let mut req = req;
            req.extensions_mut().insert(KeyIdentity {
                key_id: v.key_id,
                key_hash: v.key_hash,
                token_budget: v.token_budget,
                tokens_used: v.tokens_used,
            });
            next.run(req).await
        }
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": { "message": "Invalid API key" } })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Key validation error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": { "message": "Internal server error" } })),
            )
                .into_response()
        }
    }
}
