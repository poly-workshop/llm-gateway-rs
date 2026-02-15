use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub admin_key: String,
    pub listen_addr: String,
    /// Comma-separated list of allowed CORS origins, or "*" for any.
    pub cors_origin: String,
    /// Number of days to retain request logs. 0 = keep forever.
    pub log_retention_days: u32,
    /// Whether to store the full request body in the log.
    pub log_request_body: bool,
    /// Whether to store the full response body in the log.
    /// For SSE streaming, this enables shadow stream to capture data.
    pub log_response_body: bool,
}

fn parse_bool_env(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(v) => matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"),
        Err(_) => default,
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .map_err(|_| anyhow::anyhow!("DATABASE_URL is required"))?,
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
            admin_key: env::var("ADMIN_KEY")
                .map_err(|_| anyhow::anyhow!("ADMIN_KEY is required"))?,
            listen_addr: env::var("LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:3000".into()),
            cors_origin: env::var("CORS_ORIGIN")
                .unwrap_or_else(|_| "*".into()),
            log_retention_days: env::var("LOG_RETENTION_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7),
            log_request_body: parse_bool_env("LOG_REQUEST_BODY", false),
            log_response_body: parse_bool_env("LOG_RESPONSE_BODY", false),
        })
    }
}
