use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Full DB row for a request log entry.
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct RequestLog {
    pub id: Uuid,
    pub request_id: Option<String>,
    pub user_key_id: Option<Uuid>,
    pub user_key_hash: String,
    pub model_requested: String,
    pub model_sent: String,
    pub provider_id: Option<Uuid>,
    pub provider_kind: Option<String>,
    pub status_code: i16,
    pub is_error: bool,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub latency_ms: i32,
    pub is_stream: bool,
    pub request_body: Option<serde_json::Value>,
    pub response_body: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Public info returned by the admin logs listing API.
#[derive(Debug, Serialize)]
pub struct RequestLogInfo {
    pub id: Uuid,
    pub request_id: Option<String>,
    pub user_key_id: Option<Uuid>,
    pub model_requested: String,
    pub model_sent: String,
    pub provider_id: Option<Uuid>,
    pub provider_kind: Option<String>,
    pub status_code: i16,
    pub is_error: bool,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub latency_ms: i32,
    pub is_stream: bool,
    pub request_body: Option<serde_json::Value>,
    pub response_body: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<RequestLog> for RequestLogInfo {
    fn from(r: RequestLog) -> Self {
        Self {
            id: r.id,
            request_id: r.request_id,
            user_key_id: r.user_key_id,
            model_requested: r.model_requested,
            model_sent: r.model_sent,
            provider_id: r.provider_id,
            provider_kind: r.provider_kind,
            status_code: r.status_code,
            is_error: r.is_error,
            prompt_tokens: r.prompt_tokens,
            completion_tokens: r.completion_tokens,
            total_tokens: r.total_tokens,
            latency_ms: r.latency_ms,
            is_stream: r.is_stream,
            request_body: r.request_body,
            response_body: r.response_body,
            error_message: r.error_message,
            created_at: r.created_at,
        }
    }
}

/// Paginated response wrapper for log listing.
#[derive(Debug, Serialize)]
pub struct LogListResponse {
    pub data: Vec<RequestLogInfo>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}
