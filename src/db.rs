use anyhow::{Context, Result, anyhow};
use sqlx::{FromRow, MySql, MySqlPool, mysql::MySqlPoolOptions};
use url::Url;

#[derive(Clone)]
pub struct DatabaseConnection {
    pub pool: MySqlPool,
    pub database_name: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserRecord {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub nickname: Option<String>,
    pub school_id: Option<i64>,
    pub created_at: String,
}

pub async fn connect(database_url: Option<&str>) -> Result<Option<DatabaseConnection>> {
    let Some(database_url) = database_url else {
        return Ok(None);
    };

    let database_name = database_name_from_url(database_url)?;
    ensure_database_exists(database_url, &database_name).await?;

    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .context("MySQL 连接失败")?;

    initialize_schema(&pool).await?;

    Ok(Some(DatabaseConnection {
        pool,
        database_name,
    }))
}

pub async fn find_user_by_email(pool: &MySqlPool, email: &str) -> Result<Option<UserRecord>> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, password_hash, nickname, school_id,
               DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s') AS created_at
        FROM users
        WHERE email = ?
        LIMIT 1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .context("按邮箱查询用户失败")
}

pub async fn find_user_by_id(pool: &MySqlPool, user_id: i64) -> Result<Option<UserRecord>> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, password_hash, nickname, school_id,
               DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s') AS created_at
        FROM users
        WHERE id = ?
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .context("按 ID 查询用户失败")
}

pub async fn create_user(
    pool: &MySqlPool,
    email: &str,
    password_hash: &str,
) -> Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO users (email, password_hash)
        VALUES (?, ?)
        "#,
    )
    .bind(email)
    .bind(password_hash)
    .execute(pool)
    .await
    .context("创建用户失败")?;

    Ok(result.last_insert_id() as i64)
}

async fn ensure_database_exists(database_url: &str, database_name: &str) -> Result<()> {
    let admin_url = admin_database_url(database_url)?;
    let admin_pool = MySqlPoolOptions::new()
        .max_connections(2)
        .connect(&admin_url)
        .await
        .context("连接 MySQL 管理库失败")?;

    let statement = format!(
        "CREATE DATABASE IF NOT EXISTS `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci",
        database_name
    );
    sqlx::query(&statement)
        .execute(&admin_pool)
        .await
        .context("创建应用数据库失败")?;

    admin_pool.close().await;
    Ok(())
}

async fn initialize_schema(pool: &MySqlPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id            BIGINT PRIMARY KEY AUTO_INCREMENT,
            email         VARCHAR(255) NOT NULL UNIQUE,
            password_hash VARCHAR(255) NOT NULL,
            nickname      VARCHAR(50) NULL,
            school_id     INT NULL,
            created_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .context("创建 users 表失败")?;

    Ok(())
}

fn database_name_from_url(database_url: &str) -> Result<String> {
    let url = Url::parse(database_url).context("DATABASE_URL 不是有效的 URL")?;
    let name = url.path().trim_start_matches('/').to_string();
    if name.is_empty() {
        return Err(anyhow!("DATABASE_URL 必须包含数据库名"));
    }
    Ok(name)
}

fn admin_database_url(database_url: &str) -> Result<String> {
    let mut url = Url::parse(database_url).context("DATABASE_URL 不是有效的 URL")?;
    url.set_path("/mysql");
    Ok(url.to_string())
}
