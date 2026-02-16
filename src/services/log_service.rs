use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::request_log::{LogListResponse, RequestLogInfo};

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

/// Row struct for the joined log + model coefficients query.
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct RequestLogRow {
    // request_logs columns
    id: uuid::Uuid,
    request_id: Option<String>,
    user_key_id: Option<uuid::Uuid>,
    user_key_hash: String,
    model_requested: String,
    model_sent: String,
    provider_id: Option<uuid::Uuid>,
    provider_kind: Option<String>,
    status_code: i16,
    is_error: bool,
    prompt_tokens: Option<i32>,
    completion_tokens: Option<i32>,
    total_tokens: Option<i32>,
    latency_ms: i32,
    is_stream: bool,
    request_body: Option<serde_json::Value>,
    response_body: Option<serde_json::Value>,
    error_message: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    // computed
    weighted_total_tokens: Option<i64>,
}

impl From<RequestLogRow> for RequestLogInfo {
    fn from(r: RequestLogRow) -> Self {
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
            weighted_total_tokens: r.weighted_total_tokens,
            latency_ms: r.latency_ms,
            is_stream: r.is_stream,
            request_body: r.request_body,
            response_body: r.response_body,
            error_message: r.error_message,
            created_at: r.created_at,
        }
    }
}

/// List logs with offset-based pagination and optional filters.
pub async fn list_logs(db: &PgPool, params: ListLogsParams) -> Result<LogListResponse, AppError> {
    let offset = (params.page - 1).max(0) * params.per_page;

    // Build dynamic WHERE clauses
    let mut conditions: Vec<String> = vec![];
    if params.key_id.is_some() {
        conditions.push("r.user_key_id = $3".to_string());
    }
    if params.model.is_some() {
        let idx = if params.key_id.is_some() { 4 } else { 3 };
        conditions.push(format!("r.model_requested = ${idx}"));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_query = format!("SELECT COUNT(*) FROM request_logs r {where_clause}");
    let data_query = format!(
        r#"SELECT r.id, r.request_id, r.user_key_id, r.user_key_hash,
                  r.model_requested, r.model_sent, r.provider_id, r.provider_kind,
                  r.status_code, r.is_error, r.prompt_tokens, r.completion_tokens, r.total_tokens,
                  r.latency_ms, r.is_stream, r.request_body, r.response_body, r.error_message,
                  r.created_at,
                  CASE WHEN r.prompt_tokens IS NOT NULL OR r.completion_tokens IS NOT NULL
                       THEN ROUND(
                           COALESCE(r.prompt_tokens, 0) * COALESCE(m.input_token_coefficient, 1.0)
                           + COALESCE(r.completion_tokens, 0) * COALESCE(m.output_token_coefficient, 1.0)
                       )::BIGINT
                       ELSE NULL
                  END AS weighted_total_tokens
           FROM request_logs r
           LEFT JOIN models m ON m.name = r.model_requested
           {where_clause}
           ORDER BY r.created_at DESC
           LIMIT $1 OFFSET $2"#
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
    let rows: Vec<RequestLogRow> = {
        let mut q = sqlx::query_as::<_, RequestLogRow>(&data_query)
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
        data: rows.into_iter().map(RequestLogInfo::from).collect(),
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

// ── Dashboard Stats ───────────────────────────────────────────────────

use serde::Serialize;

/// Summary numbers for the dashboard.
#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub total_requests: i64,
    pub total_requests_24h: i64,
    pub total_errors_24h: i64,
    pub total_tokens_24h: i64,
    pub avg_latency_24h: f64,
    /// Requests per hour (last 24h). Each entry: { hour: "HH:00", requests, errors }.
    pub requests_per_hour: Vec<HourlyBucket>,
    /// Per-model request count and tokens (last 7 days).
    pub model_usage: Vec<ModelUsage>,
    /// Per-provider request count (last 7 days).
    pub provider_usage: Vec<ProviderUsage>,
}

#[derive(Debug, Serialize)]
pub struct HourlyBucket {
    pub hour: String,
    pub requests: i64,
    pub errors: i64,
    pub tokens: i64,
    pub avg_latency: f64,
}

#[derive(Debug, Serialize)]
pub struct ModelUsage {
    pub model: String,
    pub requests: i64,
    pub tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct ProviderUsage {
    pub provider: String,
    pub requests: i64,
    pub errors: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct SummaryRow {
    total_requests: Option<i64>,
    total_requests_24h: Option<i64>,
    total_errors_24h: Option<i64>,
    total_tokens_24h: Option<i64>,
    avg_latency_24h: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
struct HourlyRow {
    hour: chrono::DateTime<chrono::Utc>,
    requests: i64,
    errors: i64,
    tokens: i64,
    avg_latency: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct ModelRow {
    model: String,
    requests: i64,
    tokens: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct ProviderRow {
    provider: String,
    requests: i64,
    errors: i64,
}

pub async fn get_dashboard_stats(db: &PgPool) -> Result<DashboardStats, AppError> {
    // 1) Summary
    let summary = sqlx::query_as::<_, SummaryRow>(
        r#"
        SELECT
            COUNT(*)::BIGINT AS total_requests,
            COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '24 hours')::BIGINT AS total_requests_24h,
            COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '24 hours' AND is_error)::BIGINT AS total_errors_24h,
            COALESCE(SUM(total_tokens) FILTER (WHERE created_at >= NOW() - INTERVAL '24 hours'), 0)::BIGINT AS total_tokens_24h,
            COALESCE(AVG(latency_ms) FILTER (WHERE created_at >= NOW() - INTERVAL '24 hours'), 0)::FLOAT8 AS avg_latency_24h
        FROM request_logs
        "#,
    )
    .fetch_one(db)
    .await?;

    // 2) Hourly buckets (last 24h)
    let hourly_rows = sqlx::query_as::<_, HourlyRow>(
        r#"
        SELECT
            date_trunc('hour', created_at) AS hour,
            COUNT(*) AS requests,
            COUNT(*) FILTER (WHERE is_error) AS errors,
            COALESCE(SUM(total_tokens), 0)::BIGINT AS tokens,
            COALESCE(AVG(latency_ms), 0)::FLOAT8 AS avg_latency
        FROM request_logs
        WHERE created_at >= NOW() - INTERVAL '24 hours'
        GROUP BY date_trunc('hour', created_at)
        ORDER BY hour
        "#,
    )
    .fetch_all(db)
    .await?;

    let requests_per_hour: Vec<HourlyBucket> = hourly_rows
        .into_iter()
        .map(|r| HourlyBucket {
            hour: r.hour.format("%H:%M").to_string(),
            requests: r.requests,
            errors: r.errors,
            tokens: r.tokens,
            avg_latency: (r.avg_latency * 10.0).round() / 10.0,
        })
        .collect();

    // 3) Per-model usage (last 7 days)
    let model_rows = sqlx::query_as::<_, ModelRow>(
        r#"
        SELECT
            model_requested AS model,
            COUNT(*) AS requests,
            COALESCE(SUM(total_tokens), 0)::BIGINT AS tokens
        FROM request_logs
        WHERE created_at >= NOW() - INTERVAL '7 days'
        GROUP BY model_requested
        ORDER BY requests DESC
        LIMIT 20
        "#,
    )
    .fetch_all(db)
    .await?;

    let model_usage: Vec<ModelUsage> = model_rows
        .into_iter()
        .map(|r| ModelUsage {
            model: r.model,
            requests: r.requests,
            tokens: r.tokens,
        })
        .collect();

    // 4) Per-provider usage (last 7 days)
    let provider_rows = sqlx::query_as::<_, ProviderRow>(
        r#"
        SELECT
            COALESCE(provider_kind, 'unknown') AS provider,
            COUNT(*) AS requests,
            COUNT(*) FILTER (WHERE is_error) AS errors
        FROM request_logs
        WHERE created_at >= NOW() - INTERVAL '7 days'
        GROUP BY provider_kind
        ORDER BY requests DESC
        "#,
    )
    .fetch_all(db)
    .await?;

    let provider_usage: Vec<ProviderUsage> = provider_rows
        .into_iter()
        .map(|r| ProviderUsage {
            provider: r.provider,
            requests: r.requests,
            errors: r.errors,
        })
        .collect();

    Ok(DashboardStats {
        total_requests: summary.total_requests.unwrap_or(0),
        total_requests_24h: summary.total_requests_24h.unwrap_or(0),
        total_errors_24h: summary.total_errors_24h.unwrap_or(0),
        total_tokens_24h: summary.total_tokens_24h.unwrap_or(0),
        avg_latency_24h: (summary.avg_latency_24h.unwrap_or(0.0) * 10.0).round() / 10.0,
        requests_per_hour,
        model_usage,
        provider_usage,
    })
}
