#[macro_use]
mod db;
mod cache;
mod config;
mod error;
mod middleware;
mod models;
mod routes;
mod services;
mod state;

use std::sync::Arc;

use axum::{http::HeaderValue, middleware as axum_mw, Router};
use tokio::net::TcpListener;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use cache::Cache;
use config::Config;
use db::DbPool;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file (ignore if missing)
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Load config
    let config = Config::from_env()?;
    tracing::info!("Starting LLM Gateway on {}", config.listen_addr);

    // Create database pool (Postgres or SQLite)
    let db = if config.is_sqlite() {
        tracing::info!("Using SQLite database");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&config.database_url)
            .await?;

        // Enable WAL mode and foreign keys for better performance and correctness
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;

        DbPool::Sqlite(pool)
    } else {
        tracing::info!("Using PostgreSQL database");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(&config.database_url)
            .await?;
        DbPool::Pg(pool)
    };

    // Run migrations (select the correct set based on database kind)
    match &db {
        DbPool::Pg(pool) => {
            sqlx::migrate!("./migrations").run(pool).await?;
        }
        DbPool::Sqlite(pool) => {
            sqlx::migrate!("./migrations_sqlite").run(pool).await?;
        }
    }
    tracing::info!("Database migrations applied");

    // Create cache (Redis or in-memory)
    let mut cache = if let Some(ref redis_url) = config.redis_url {
        let redis_client = redis::Client::open(redis_url.as_str())?;
        let cm = redis_client.get_connection_manager().await?;
        tracing::info!("Connected to Redis");
        Cache::Redis(cm)
    } else {
        tracing::info!("Using in-memory cache (single-instance mode)");
        Cache::in_memory()
    };

    // Warm up caches
    services::key_service::warm_up_cache(&db, &mut cache).await?;
    services::model_service::warm_up_model_routes(&db, &mut cache).await?;

    // Build shared state
    let state = Arc::new(AppState {
        db,
        cache,
        config: config.clone(),
        http_client: reqwest::Client::new(),
    });

    // Spawn background log retention task
    if config.log_retention_days > 0 {
        let retention_db = state.db.clone();
        let retention_days = config.log_retention_days;
        tokio::spawn(async move {
            // Run cleanup once on startup, then every hour
            loop {
                match services::log_service::cleanup_old_logs(&retention_db, retention_days).await {
                    Ok(n) if n > 0 => {
                        tracing::info!(
                            "Cleaned up {} request logs older than {} days",
                            n,
                            retention_days
                        );
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Log cleanup error: {}", e);
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
    }

    // Build routes
    let admin_routes = routes::admin::router()
        .route_layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::auth::admin_auth,
        ));

    let proxy_routes = routes::proxy::router()
        .route_layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::auth::user_key_auth,
        ));

    let allow_origin = if config.cors_origin == "*" {
        AllowOrigin::any()
    } else {
        let origins: Vec<HeaderValue> = config
            .cors_origin
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        AllowOrigin::list(origins)
    };

    let cors = CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    let app = Router::new()
        .nest("/admin", admin_routes)
        .nest("/v1", proxy_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let listener = TcpListener::bind(&config.listen_addr).await?;
    tracing::info!("Listening on {}", config.listen_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
