use std::{env, net::SocketAddr};

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: Option<String>,
    pub jwt_secret: String,
    pub jwt_expires_seconds: u64,
    /// 文件上传保存目录（相对路径相对于程序工作目录）
    pub upload_dir: String,
    /// 访问上传文件的公开基础 URL（如 https://example.com），不配置则用 http://{host}:{port}
    pub public_base_url: Option<String>,
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

        let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "uploads".to_string());
        let public_base_url = env::var("PUBLIC_BASE_URL")
            .ok()
            .filter(|v| !v.is_empty());

        Ok(Self {
            host,
            port,
            database_url,
            jwt_secret,
            jwt_expires_seconds,
            upload_dir,
            public_base_url,
        })
    }

    pub fn addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .context("APP_HOST 或 APP_PORT 无效")
    }

    /// 构造上传文件的完整可访问 URL
    pub fn public_url(&self, relative_path: &str) -> String {
        let base = self
            .public_base_url
            .clone()
            .unwrap_or_else(|| format!("http://{}:{}", self.host, self.port));
        let path = relative_path.trim_start_matches('/');
        format!("{}/{}", base.trim_end_matches('/'), path)
    }
}
