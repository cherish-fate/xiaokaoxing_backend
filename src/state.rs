use std::sync::Arc;

use sqlx::MySqlPool;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: Option<MySqlPool>,
    pub database_name: Option<String>,
}
