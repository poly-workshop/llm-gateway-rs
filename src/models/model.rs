use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct Model {
    pub id: Uuid,
    pub name: String,
    pub provider_id: Uuid,
    pub provider_model_name: Option<String>,
    pub is_active: bool,
    pub input_token_coefficient: f64,
    pub output_token_coefficient: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public info returned by list/get.
#[derive(Debug, Serialize)]
pub struct ModelInfo {
    pub id: Uuid,
    pub name: String,
    pub provider_id: Uuid,
    pub provider_name: Option<String>,
    pub provider_model_name: Option<String>,
    pub is_active: bool,
    pub input_token_coefficient: f64,
    pub output_token_coefficient: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The resolved routing information for a model — used by the proxy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoute {
    /// Provider UUID for logging
    pub provider_id: uuid::Uuid,
    /// The model name to send to the provider (may differ from user-facing name)
    pub provider_model_name: String,
    /// Provider base URL (e.g. "https://api.openai.com/v1")
    pub base_url: String,
    /// Provider API key
    pub api_key: String,
    /// Provider kind for any provider-specific behavior
    pub provider_kind: String,
    /// Input (prompt) token cost coefficient (default 1.0)
    pub input_token_coefficient: f64,
    /// Output (completion) token cost coefficient (default 1.0)
    pub output_token_coefficient: f64,
}
