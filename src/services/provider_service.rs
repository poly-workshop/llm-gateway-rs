use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::provider::{Provider, ProviderInfo, ProviderKind};

/// Create a new provider.
pub async fn create_provider(
    name: &str,
    kind: &str,
    base_url: Option<&str>,
    api_key: &str,
    db: &PgPool,
) -> Result<ProviderInfo, AppError> {
    let pk = ProviderKind::from_str(kind)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown provider kind: {kind}. Supported: openai, openrouter, dashscope, ark")))?;

    let resolved_base_url = base_url.unwrap_or_else(|| pk.default_base_url());
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO providers (id, name, kind, base_url, api_key, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, TRUE, $6, $6)
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(pk.as_str())
    .bind(resolved_base_url)
    .bind(api_key)
    .bind(now)
    .execute(db)
    .await?;

    let provider = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = $1")
        .bind(id)
        .fetch_one(db)
        .await?;

    Ok(ProviderInfo::from(provider))
}

/// List all providers.
pub async fn list_providers(db: &PgPool) -> Result<Vec<ProviderInfo>, AppError> {
    let providers = sqlx::query_as::<_, Provider>("SELECT * FROM providers ORDER BY created_at DESC")
        .fetch_all(db)
        .await?;

    Ok(providers.into_iter().map(ProviderInfo::from).collect())
}

/// Update a provider.
pub async fn update_provider(
    id: Uuid,
    name: Option<&str>,
    kind: Option<&str>,
    base_url: Option<&str>,
    api_key: Option<&str>,
    is_active: Option<bool>,
    db: &PgPool,
) -> Result<ProviderInfo, AppError> {
    let existing = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound)?;

    let new_kind = match kind {
        Some(k) => {
            ProviderKind::from_str(k)
                .ok_or_else(|| AppError::BadRequest(format!("Unknown provider kind: {k}")))?;
            k.to_lowercase()
        }
        None => existing.kind,
    };

    let new_name = name.map(|s| s.to_string()).unwrap_or(existing.name);
    let new_base_url = base_url.map(|s| s.to_string()).unwrap_or(existing.base_url);
    let new_api_key = api_key.map(|s| s.to_string()).unwrap_or(existing.api_key);
    let new_is_active = is_active.unwrap_or(existing.is_active);

    sqlx::query(
        r#"
        UPDATE providers
        SET name = $1, kind = $2, base_url = $3, api_key = $4, is_active = $5, updated_at = NOW()
        WHERE id = $6
        "#,
    )
    .bind(&new_name)
    .bind(&new_kind)
    .bind(&new_base_url)
    .bind(&new_api_key)
    .bind(new_is_active)
    .bind(id)
    .execute(db)
    .await?;

    let updated = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = $1")
        .bind(id)
        .fetch_one(db)
        .await?;

    Ok(ProviderInfo::from(updated))
}

/// Delete a provider (hard delete — will fail if models reference it).
pub async fn delete_provider(id: Uuid, db: &PgPool) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM providers WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(())
}
