use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Extension, Router,
};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::middleware::auth::KeyIdentity;
use crate::services::{key_service, log_service, model_service};
use crate::state::AppState;

type ByteChunk = Vec<u8>;

/// POST /v1/chat/completions — proxy to the provider resolved from the model name
async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Extension(key_identity): Extension<KeyIdentity>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response, Response> {
    let start = Instant::now();

    // Parse body to extract model name and stream flag
    let mut body_json: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "error": { "message": format!("Invalid JSON: {e}") } })),
            )
                .into_response()
        })?;

    let model_name = body_json
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "error": { "message": "\"model\" field is required" } })),
            )
                .into_response()
        })?
        .to_string();

    let is_stream = body_json
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Check token budget before proxying
    if let Some(budget) = key_identity.token_budget {
        if key_identity.tokens_used >= budget {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                axum::Json(serde_json::json!({
                    "error": {
                        "message": format!(
                            "Token budget exhausted: {}/{} tokens used",
                            key_identity.tokens_used, budget
                        )
                    }
                })),
            )
                .into_response());
        }
    }

    // Resolve model → provider routing
    let mut redis = state.redis.clone();
    let route = model_service::resolve_model_route(&model_name, &mut redis, &state.db)
        .await
        .map_err(|e| {
            tracing::error!("Model route resolution error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": { "message": "Internal server error" } })),
            )
                .into_response()
        })?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": { "message": format!("Model \"{model_name}\" is not configured in the gateway") }
                })),
            )
                .into_response()
        })?;

    // Capture log context
    let log_request_body = state.config.log_request_body;
    let log_response_body = state.config.log_response_body;
    let saved_request_body = if log_request_body {
        Some(body_json.clone())
    } else {
        None
    };

    // Rewrite model name if the provider uses a different name
    let model_sent = route.provider_model_name.clone();
    if route.provider_model_name != model_name {
        body_json["model"] = serde_json::Value::String(route.provider_model_name.clone());
    }

    // For streaming requests, inject stream_options to request usage data
    // Many OpenAI-compatible providers only include usage when this is set
    if is_stream {
        if body_json.get("stream_options").is_none() {
            body_json["stream_options"] = serde_json::json!({ "include_usage": true });
        }
    }

    let upstream_body = serde_json::to_vec(&body_json).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": { "message": format!("JSON serialization error: {e}") } })),
        )
            .into_response()
    })?;

    // Build upstream URL
    let url = format!("{}/chat/completions", route.base_url);

    // Build the upstream request with provider-specific auth
    let mut upstream_req = state
        .http_client
        .post(&url)
        .header(header::AUTHORIZATION, format!("Bearer {}", route.api_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(upstream_body);

    // Provider-specific headers
    match route.provider_kind.as_str() {
        "openrouter" => {
            if let Some(referer) = headers.get("http-referer") {
                upstream_req = upstream_req.header("HTTP-Referer", referer);
            }
            if let Some(title) = headers.get("x-title") {
                upstream_req = upstream_req.header("X-Title", title);
            }
        }
        _ => {
            if let Some(org) = headers.get("openai-organization") {
                upstream_req = upstream_req.header("OpenAI-Organization", org);
            }
        }
    }

    let upstream_resp = upstream_req.send().await.map_err(|e| {
        tracing::error!("Upstream request to {} failed: {}", route.provider_kind, e);
        (
            StatusCode::BAD_GATEWAY,
            axum::Json(serde_json::json!({ "error": { "message": "Upstream service error" } })),
        )
            .into_response()
    })?;

    let status =
        StatusCode::from_u16(upstream_resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let is_error = !status.is_success();

    // Extract upstream request-id if present
    let request_id = upstream_resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if is_stream {
        let upstream_headers = upstream_resp.headers().clone();

        // Always use shadow stream for SSE to capture usage/tokens regardless of log_response_body setting
        let (shadow_tx, shadow_rx) = mpsc::unbounded_channel::<ByteChunk>();

        let raw_stream = upstream_resp.bytes_stream();

        let shadow_stream = ShadowStream {
            inner: Box::pin(raw_stream),
            tx: shadow_tx,
        };

        let body = Body::from_stream(shadow_stream);

        let mut response = Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .header(header::CACHE_CONTROL, "no-cache")
            .header(header::CONNECTION, "keep-alive")
            .body(body)
            .unwrap();

        copy_upstream_headers(&upstream_headers, response.headers_mut());

        // Spawn background task to accumulate shadow chunks, parse usage, and log
        let db = state.db.clone();
        let log_model_requested = model_name.clone();
        let log_model_sent = model_sent.clone();
        let log_provider_id = route.provider_id;
        let log_provider_kind = route.provider_kind.clone();
        let log_key_identity = key_identity.clone();
        let log_request_id = request_id.clone();
        let log_status = status.as_u16() as i16;
        let log_is_error = is_error;

        tokio::spawn(async move {
            let mut buffer = Vec::new();
            let mut shadow_rx = shadow_rx;
            while let Some(chunk) = shadow_rx.recv().await {
                buffer.extend_from_slice(&chunk);
            }

            let latency_ms = start.elapsed().as_millis() as i32;

            // Parse SSE buffer to extract usage
            let (prompt_tokens, completion_tokens, total_tokens, response_body_json) =
                parse_sse_usage_and_body(&buffer);

            // Only store response body if configured
            let saved_response = if log_response_body { response_body_json } else { None };

            if let Err(e) = log_service::insert_log(
                &db,
                log_service::NewRequestLog {
                    request_id: log_request_id,
                    user_key_id: Some(log_key_identity.key_id),
                    user_key_hash: log_key_identity.key_hash,
                    model_requested: log_model_requested,
                    model_sent: log_model_sent,
                    provider_id: Some(log_provider_id),
                    provider_kind: Some(log_provider_kind),
                    status_code: log_status,
                    is_error: log_is_error,
                    prompt_tokens,
                    completion_tokens,
                    total_tokens,
                    latency_ms,
                    is_stream: true,
                    request_body: saved_request_body,
                    response_body: saved_response,
                    error_message: None,
                },
            )
            .await
            {
                tracing::error!("Failed to insert request log: {}", e);
            }

            // Increment token usage
            if let Some(tokens) = total_tokens {
                if tokens > 0 {
                    if let Err(e) = key_service::increment_tokens_used(
                        log_key_identity.key_id, tokens as i64, &db,
                    ).await {
                        tracing::error!("Failed to increment token usage: {}", e);
                    }
                }
            }
        });

        Ok(response)
    } else {
        // Non-streaming response
        let upstream_headers = upstream_resp.headers().clone();
        let response_bytes = upstream_resp.bytes().await.map_err(|e| {
            tracing::error!("Failed to read upstream response: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                axum::Json(serde_json::json!({ "error": { "message": "Failed to read upstream response" } })),
            )
                .into_response()
        })?;

        // Parse usage from response body (always, since it's cheap)
        let resp_json: Option<serde_json::Value> =
            serde_json::from_slice(&response_bytes).ok();

        let (prompt_tokens, completion_tokens, total_tokens) = resp_json
            .as_ref()
            .and_then(|j| j.get("usage"))
            .map(|usage| {
                (
                    usage.get("prompt_tokens").and_then(|v| v.as_i64()).map(|v| v as i32),
                    usage.get("completion_tokens").and_then(|v| v.as_i64()).map(|v| v as i32),
                    usage.get("total_tokens").and_then(|v| v.as_i64()).map(|v| v as i32),
                )
            })
            .unwrap_or((None, None, None));

        let error_message = if is_error {
            resp_json
                .as_ref()
                .and_then(|j| j.get("error"))
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        let saved_response_body = if log_response_body { resp_json } else { None };

        let mut response = Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(response_bytes))
            .unwrap();

        copy_upstream_headers(&upstream_headers, response.headers_mut());

        // Async log insert
        let db = state.db.clone();
        let latency_ms = start.elapsed().as_millis() as i32;
        let log_key_id = key_identity.key_id;
        tokio::spawn(async move {
            if let Err(e) = log_service::insert_log(
                &db,
                log_service::NewRequestLog {
                    request_id,
                    user_key_id: Some(key_identity.key_id),
                    user_key_hash: key_identity.key_hash,
                    model_requested: model_name,
                    model_sent,
                    provider_id: Some(route.provider_id),
                    provider_kind: Some(route.provider_kind),
                    status_code: status.as_u16() as i16,
                    is_error,
                    prompt_tokens,
                    completion_tokens,
                    total_tokens,
                    latency_ms,
                    is_stream: false,
                    request_body: saved_request_body,
                    response_body: saved_response_body,
                    error_message,
                },
            )
            .await
            {
                tracing::error!("Failed to insert request log: {}", e);
            }

            // Increment token usage
            if let Some(tokens) = total_tokens {
                if tokens > 0 {
                    if let Err(e) = key_service::increment_tokens_used(
                        log_key_id, tokens as i64, &db,
                    ).await {
                        tracing::error!("Failed to increment token usage: {}", e);
                    }
                }
            }
        });

        Ok(response)
    }
}

// ── Shadow Stream ─────────────────────────────────────────────────────

use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A stream wrapper that yields chunks to the client while sending copies
/// to a background channel for aggregation (shadow stream).
struct ShadowStream {
    inner: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    tx: mpsc::UnboundedSender<ByteChunk>,
}

impl Stream for ShadowStream {
    type Item = Result<bytes::Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                // Send a copy to the shadow channel (ignore errors if receiver dropped)
                let _ = self.tx.send(chunk.to_vec());
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(std::io::Error::new(std::io::ErrorKind::Other, e))))
            }
            Poll::Ready(None) => {
                // Stream ended — drop the sender so the receiver knows
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

// ── SSE Usage Parser ──────────────────────────────────────────────────

/// Parse concatenated SSE bytes to extract `usage` from any `data:` event.
/// Scans all chunks and keeps the last `usage` object found (providers may place
/// it on the final content chunk, a separate chunk, or both).
/// Returns (prompt_tokens, completion_tokens, total_tokens, optional full response body).
fn parse_sse_usage_and_body(
    buffer: &[u8],
) -> (Option<i32>, Option<i32>, Option<i32>, Option<serde_json::Value>) {
    let text = String::from_utf8_lossy(buffer);

    let mut all_chunks: Vec<serde_json::Value> = Vec::new();
    let mut usage_prompt: Option<i32> = None;
    let mut usage_completion: Option<i32> = None;
    let mut usage_total: Option<i32> = None;

    for line in text.lines() {
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if data == "[DONE]" {
                continue;
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                // Check for usage in this chunk (keep latest found)
                if let Some(usage) = json.get("usage") {
                    if let Some(pt) = usage.get("prompt_tokens").and_then(|v| v.as_i64()) {
                        usage_prompt = Some(pt as i32);
                    }
                    if let Some(ct) = usage.get("completion_tokens").and_then(|v| v.as_i64()) {
                        usage_completion = Some(ct as i32);
                    }
                    if let Some(tt) = usage.get("total_tokens").and_then(|v| v.as_i64()) {
                        usage_total = Some(tt as i32);
                    }
                }
                all_chunks.push(json);
            }
        }
    }

    // Build a response body from the chunks for storage
    let response_body = if all_chunks.is_empty() {
        None
    } else {
        Some(serde_json::Value::Array(all_chunks))
    };

    (usage_prompt, usage_completion, usage_total, response_body)
}

// ── Helpers ───────────────────────────────────────────────────────────

/// Copy useful upstream headers (rate-limit, request-id, etc.) to the gateway response.
fn copy_upstream_headers(from: &reqwest::header::HeaderMap, to: &mut HeaderMap) {
    let headers_to_copy = [
        "x-ratelimit-limit-requests",
        "x-ratelimit-limit-tokens",
        "x-ratelimit-remaining-requests",
        "x-ratelimit-remaining-tokens",
        "x-ratelimit-reset-requests",
        "x-ratelimit-reset-tokens",
        "x-request-id",
        "openai-processing-ms",
        "openai-organization",
    ];

    for name in headers_to_copy {
        if let Some(val) = from.get(name) {
            if let Ok(v) = HeaderValue::from_bytes(val.as_bytes()) {
                to.insert(
                    header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                    v,
                );
            }
        }
    }
}

/// Build the proxy router (to be nested under /v1)
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/chat/completions", post(chat_completions))
}
