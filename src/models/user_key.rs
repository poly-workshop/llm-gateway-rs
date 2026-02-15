use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct UserKey {
    pub id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response when listing keys — never exposes hash or full key
#[derive(Debug, Serialize)]
pub struct UserKeyInfo {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UserKey> for UserKeyInfo {
    fn from(k: UserKey) -> Self {
        Self {
            id: k.id,
            name: k.name,
            key_prefix: k.key_prefix,
            is_active: k.is_active,
            created_at: k.created_at,
            updated_at: k.updated_at,
        }
    }
}

/// Response when creating or rotating a key — includes the plaintext key (shown only once)
#[derive(Debug, Serialize)]
pub struct UserKeyCreated {
    pub id: Uuid,
    pub name: String,
    pub key: String,
    pub key_prefix: String,
    pub created_at: DateTime<Utc>,
}
