use chrono::Utc;
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::provider::{Provider, ProviderInfo, ProviderKind};

/// Create a new provider.
pub async fn create_provider(
    name: &str,
    kind: &str,
    base_url: Option<&str>,
    api_key: &str,
    db: &DbPool,
) -> Result<ProviderInfo, AppError> {
    let pk = ProviderKind::from_str(kind)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown provider kind: {kind}. Supported: openai, openrouter, dashscope, ark")))?;

    let resolved_base_url = base_url.unwrap_or_else(|| pk.default_base_url());
    let id = Uuid::new_v4();
    let now = Utc::now();

    db_execute!(
        db,
        r#"
        INSERT INTO providers (id, name, kind, base_url, api_key, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, TRUE, $6, $6)
        "#,
        id, name, pk.as_str(), resolved_base_url, api_key, now
    )?;

    let provider: Provider = db_query_as!(
        one, db,
        "SELECT * FROM providers WHERE id = $1",
        id
    )?;

    Ok(ProviderInfo::from(provider))
}

/// List all providers.
pub async fn list_providers(db: &DbPool) -> Result<Vec<ProviderInfo>, AppError> {
    let providers: Vec<Provider> = db_query_as!(
        all, db,
        "SELECT * FROM providers ORDER BY created_at DESC"
    )?;

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
    db: &DbPool,
) -> Result<ProviderInfo, AppError> {
    let existing: Provider = db_query_as!(
        optional, db,
        "SELECT * FROM providers WHERE id = $1",
        id
    )?
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
    let now = Utc::now();

    db_execute!(
        db,
        r#"
        UPDATE providers
        SET name = $1, kind = $2, base_url = $3, api_key = $4, is_active = $5, updated_at = $6
        WHERE id = $7
        "#,
        &new_name, &new_kind, &new_base_url, &new_api_key, new_is_active, now, id
    )?;

    let updated: Provider = db_query_as!(
        one, db,
        "SELECT * FROM providers WHERE id = $1",
        id
    )?;

    Ok(ProviderInfo::from(updated))
}

/// Delete a provider (hard delete — will fail if models reference it).
pub async fn delete_provider(id: Uuid, db: &DbPool) -> Result<(), AppError> {
    let rows = db_execute!(db, "DELETE FROM providers WHERE id = $1", id)?;

    if rows == 0 {
        return Err(AppError::NotFound);
    }

    Ok(())
}
