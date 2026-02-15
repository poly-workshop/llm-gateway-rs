use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::request_log::{LogListResponse, RequestLog, RequestLogInfo};

/// Parameters for inserting a new log entry (built by the proxy).
pub struct NewRequestLog {
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
}

/// Insert a request log entry into the database.
pub async fn insert_log(db: &PgPool, log: NewRequestLog) -> Result<(), AppError> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO request_logs (
            id, request_id, user_key_id, user_key_hash,
            model_requested, model_sent, provider_id, provider_kind,
            status_code, is_error, prompt_tokens, completion_tokens, total_tokens,
            latency_ms, is_stream, request_body, response_body, error_message, created_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
            $14, $15, $16, $17, $18, $19
        )
        "#,
    )
    .bind(id)
    .bind(&log.request_id)
    .bind(log.user_key_id)
    .bind(&log.user_key_hash)
    .bind(&log.model_requested)
    .bind(&log.model_sent)
    .bind(log.provider_id)
    .bind(&log.provider_kind)
    .bind(log.status_code)
    .bind(log.is_error)
    .bind(log.prompt_tokens)
    .bind(log.completion_tokens)
    .bind(log.total_tokens)
    .bind(log.latency_ms)
    .bind(log.is_stream)
    .bind(&log.request_body)
    .bind(&log.response_body)
    .bind(&log.error_message)
    .bind(now)
    .execute(db)
    .await?;

    Ok(())
}

/// Query parameters for listing logs.
pub struct ListLogsParams {
    pub page: i64,
    pub per_page: i64,
    pub key_id: Option<Uuid>,
    pub model: Option<String>,
}

/// List logs with offset-based pagination and optional filters.
pub async fn list_logs(db: &PgPool, params: ListLogsParams) -> Result<LogListResponse, AppError> {
    let offset = (params.page - 1).max(0) * params.per_page;

    // Build dynamic WHERE clauses
    let mut conditions: Vec<String> = vec![];
    if params.key_id.is_some() {
        conditions.push("user_key_id = $3".to_string());
    }
    if params.model.is_some() {
        let idx = if params.key_id.is_some() { 4 } else { 3 };
        conditions.push(format!("model_requested = ${idx}"));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_query = format!("SELECT COUNT(*) FROM request_logs {where_clause}");
    let data_query = format!(
        "SELECT * FROM request_logs {where_clause} ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    );

    // Execute count query
    let total: i64 = {
        let mut q = sqlx::query_scalar::<_, i64>(&count_query);
        if let Some(ref kid) = params.key_id {
            q = q.bind(kid);
        }
        if let Some(ref m) = params.model {
            q = q.bind(m);
        }
        q.fetch_one(db).await?
    };

    // Execute data query
    let logs: Vec<RequestLog> = {
        let mut q = sqlx::query_as::<_, RequestLog>(&data_query)
            .bind(params.per_page)
            .bind(offset);
        if let Some(ref kid) = params.key_id {
            q = q.bind(kid);
        }
        if let Some(ref m) = params.model {
            q = q.bind(m);
        }
        q.fetch_all(db).await?
    };

    Ok(LogListResponse {
        data: logs.into_iter().map(RequestLogInfo::from).collect(),
        total,
        page: params.page,
        per_page: params.per_page,
    })
}

/// Delete request logs older than `retention_days` days.
/// Returns the number of rows deleted.
pub async fn cleanup_old_logs(db: &PgPool, retention_days: u32) -> Result<u64, AppError> {
    let result = sqlx::query(
        "DELETE FROM request_logs WHERE created_at < NOW() - make_interval(days => $1)",
    )
    .bind(retention_days as i32)
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}
