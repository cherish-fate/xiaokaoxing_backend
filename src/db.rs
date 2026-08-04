use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};

/// 数据库连接封装
pub struct DbConnection {
    pub pool: MySqlPool,
    pub database_name: String,
}

/// 连接 MySQL 数据库，返回 None 表示未配置 DATABASE_URL
pub async fn connect(database_url: Option<&str>) -> Result<Option<DbConnection>> {
    let Some(url) = database_url else {
        tracing::warn!("未配置 DATABASE_URL，跳过数据库连接");
        return Ok(None);
    };

    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(url)
        .await
        .context("数据库连接失败")?;

    // 从 URL 中提取数据库名称
    let database_name = url
        .rsplit('/')
        .next()
        .map(|s| s.split('?').next().unwrap_or(s))
        .unwrap_or("unknown")
        .to_string();

    tracing::info!("已连接数据库: {}", database_name);
    Ok(Some(DbConnection { pool, database_name }))
}

// ============ 模型 ============

/// 专业表 majors 对应的模型
#[derive(sqlx::FromRow)]
pub struct Major {
    pub id: i32,
    pub name: String,
}

/// 用户表 users 对应的模型（包含密码哈希，仅内部使用）
#[derive(sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub nickname: String,
    pub email: String,
    pub password_hash: String,
    pub school_name: String,
    pub major_id: i32,
    pub avatar_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ============ 专业查询 ============

/// 查询所有专业，按 id 升序
pub async fn find_all_majors(pool: &MySqlPool) -> Result<Vec<Major>> {
    let majors = sqlx::query_as::<_, Major>("SELECT id, name FROM majors ORDER BY id")
        .fetch_all(pool)
        .await
        .context("查询专业列表失败")?;
    Ok(majors)
}

/// 根据 id 查询专业，返回 None 表示不存在
pub async fn find_major_by_id(pool: &MySqlPool, id: i32) -> Result<Option<Major>> {
    let major = sqlx::query_as::<_, Major>("SELECT id, name FROM majors WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("查询专业失败")?;
    Ok(major)
}

// ============ 用户查询 ============

/// 根据邮箱查询用户（用于注册时判重、登录时查找）
pub async fn find_user_by_email(pool: &MySqlPool, email: &str) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, nickname, email, password_hash, school_name, major_id, avatar_url, created_at, updated_at FROM users WHERE email = ?",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .context("查询用户失败")?;
    Ok(user)
}

/// 创建新用户，返回完整的 User 对象
pub async fn create_user(
    pool: &MySqlPool,
    nickname: &str,
    email: &str,
    password_hash: &str,
    school_name: &str,
    major_id: i32,
) -> Result<User> {
    let result = sqlx::query(
        "INSERT INTO users (nickname, email, password_hash, school_name, major_id) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(nickname)
    .bind(email)
    .bind(password_hash)
    .bind(school_name)
    .bind(major_id)
    .execute(pool)
    .await
    .context("插入用户失败")?;

    let user_id = result.last_insert_id() as i32;

    // 查询刚创建的用户（包含 created_at 等默认值）
    let user = sqlx::query_as::<_, User>(
        "SELECT id, nickname, email, password_hash, school_name, major_id, avatar_url, created_at, updated_at FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .context("查询新创建用户失败")?;

    Ok(user)
}
