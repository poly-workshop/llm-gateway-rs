use crate::cache::Cache;
use crate::config::Config;
use crate::db::DbPool;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub cache: Cache,
    pub config: Config,
    pub http_client: reqwest::Client,
}
