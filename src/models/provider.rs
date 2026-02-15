use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Supported provider kinds.
/// All use OpenAI-compatible chat completions format, but differ in base URL and headers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    OpenAI,
    OpenRouter,
    DashScope,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::OpenAI => "openai",
            ProviderKind::OpenRouter => "openrouter",
            ProviderKind::DashScope => "dashscope",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(ProviderKind::OpenAI),
            "openrouter" => Some(ProviderKind::OpenRouter),
            "dashscope" => Some(ProviderKind::DashScope),
            _ => None,
        }
    }

    /// Default base URL for each provider kind.
    pub fn default_base_url(&self) -> &'static str {
        match self {
            ProviderKind::OpenAI => "https://api.openai.com/v1",
            ProviderKind::OpenRouter => "https://openrouter.ai/api/v1",
            ProviderKind::DashScope => "https://dashscope.aliyuncs.com/compatible-mode/v1",
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Provider {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub base_url: String,
    pub api_key: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public info returned by list/get — never exposes the full api_key.
#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub base_url: String,
    pub api_key_preview: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Provider> for ProviderInfo {
    fn from(p: Provider) -> Self {
        let preview = if p.api_key.len() > 8 {
            format!("{}...{}", &p.api_key[..4], &p.api_key[p.api_key.len() - 4..])
        } else {
            "****".to_string()
        };
        Self {
            id: p.id,
            name: p.name,
            kind: p.kind,
            base_url: p.base_url,
            api_key_preview: preview,
            is_active: p.is_active,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}
