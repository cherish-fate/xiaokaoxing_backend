use std::{env, net::SocketAddr};

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: Option<String>,
    pub jwt_secret: String,
    pub jwt_expires_seconds: u64,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let host = env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("APP_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .context("APP_PORT 必须是有效的端口号")?;
        let database_url = env::var("DATABASE_URL")
            .ok()
            .filter(|v| !v.is_empty());
        let jwt_secret =
            env::var("JWT_SECRET").unwrap_or_else(|_| "xkx-dev-secret-change-me".to_string());
        let jwt_expires_seconds = env::var("JWT_EXPIRES_SECONDS")
            .unwrap_or_else(|_| "604800".to_string())
            .parse::<u64>()
            .context("JWT_EXPIRES_SECONDS 必须是有效整数")?;

        Ok(Self {
            host,
            port,
            database_url,
            jwt_secret,
            jwt_expires_seconds,
        })
    }

    pub fn addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .context("APP_HOST 或 APP_PORT 无效")
    }
}
