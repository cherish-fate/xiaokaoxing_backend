use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use sqlx::{Row, mysql::{MySqlPool, MySqlPoolOptions}};

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
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
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

// ============ 用户查询（扩展） ============

/// 根据 ID 查询用户
pub async fn find_user_by_id(pool: &MySqlPool, id: i32) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, nickname, email, password_hash, school_name, major_id, avatar_url, created_at, updated_at FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("查询用户失败")?;
    Ok(user)
}

// ============ 考试模型 ============

#[derive(sqlx::FromRow)]
pub struct Exam {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub exam_date: chrono::NaiveDate,
    pub start_time: chrono::NaiveTime,
    pub end_time: Option<chrono::NaiveTime>,
    pub location: Option<String>,
    pub is_completed: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

// ============ 考试查询 ============

pub async fn find_exams_by_user(
    pool: &MySqlPool,
    user_id: i32,
    status: Option<&str>,
) -> Result<Vec<Exam>> {
    let today = chrono::Local::now().naive_local().date();

    let exams = if let Some(st) = status {
        match st {
            "upcoming" => {
                sqlx::query_as::<_, Exam>(
                    "SELECT id, user_id, name, exam_date, start_time, end_time, location, is_completed, created_at, updated_at FROM exams WHERE user_id = ? AND is_completed = 0 AND exam_date > ? ORDER BY exam_date ASC",
                )
                .bind(user_id)
                .bind(today)
                .fetch_all(pool)
                .await
                .context("查询考试列表失败")?
            }
            "today" => {
                sqlx::query_as::<_, Exam>(
                    "SELECT id, user_id, name, exam_date, start_time, end_time, location, is_completed, created_at, updated_at FROM exams WHERE user_id = ? AND exam_date = ? ORDER BY exam_date ASC",
                )
                .bind(user_id)
                .bind(today)
                .fetch_all(pool)
                .await
                .context("查询考试列表失败")?
            }
            "completed" => {
                sqlx::query_as::<_, Exam>(
                    "SELECT id, user_id, name, exam_date, start_time, end_time, location, is_completed, created_at, updated_at FROM exams WHERE user_id = ? AND is_completed = 1 ORDER BY exam_date ASC",
                )
                .bind(user_id)
                .fetch_all(pool)
                .await
                .context("查询考试列表失败")?
            }
            _ => {
                sqlx::query_as::<_, Exam>(
                    "SELECT id, user_id, name, exam_date, start_time, end_time, location, is_completed, created_at, updated_at FROM exams WHERE user_id = ? ORDER BY exam_date ASC",
                )
                .bind(user_id)
                .fetch_all(pool)
                .await
                .context("查询考试列表失败")?
            }
        }
    } else {
        sqlx::query_as::<_, Exam>(
            "SELECT id, user_id, name, exam_date, start_time, end_time, location, is_completed, created_at, updated_at FROM exams WHERE user_id = ? ORDER BY exam_date ASC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .context("查询考试列表失败")?
    };

    Ok(exams)
}

pub async fn find_exam_by_id(pool: &MySqlPool, id: i32) -> Result<Option<Exam>> {
    let exam = sqlx::query_as::<_, Exam>(
        "SELECT id, user_id, name, exam_date, start_time, end_time, location, is_completed, created_at, updated_at FROM exams WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("查询考试失败")?;
    Ok(exam)
}

pub async fn create_exam(
    pool: &MySqlPool,
    user_id: i32,
    name: &str,
    exam_date: NaiveDate,
    start_time: NaiveTime,
    end_time: Option<NaiveTime>,
    location: Option<&str>,
    is_completed: bool,
) -> Result<Exam> {
    let result = sqlx::query(
        "INSERT INTO exams (user_id, name, exam_date, start_time, end_time, location, is_completed) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(name)
    .bind(exam_date)
    .bind(start_time)
    .bind(end_time)
    .bind(location)
    .bind(is_completed)
    .execute(pool)
    .await
    .context("插入考试失败")?;

    let exam_id = result.last_insert_id() as i32;

    let exam = find_exam_by_id(pool, exam_id)
        .await?
        .context("查询新创建考试失败")?;
    Ok(exam)
}

pub async fn update_exam_fields(
    pool: &MySqlPool,
    id: i32,
    name: Option<&str>,
    exam_date: Option<NaiveDate>,
    start_time: Option<NaiveTime>,
    end_time: Option<NaiveTime>,
    location: Option<&str>,
    is_completed: Option<bool>,
) -> Result<Exam> {
    let exam = find_exam_by_id(pool, id)
        .await?
        .context("考试不存在")?;

    let final_name = name.unwrap_or(&exam.name);
    let final_date = exam_date.unwrap_or(exam.exam_date);
    let final_start = start_time.unwrap_or(exam.start_time);
    let final_end = end_time.or(exam.end_time);
    let final_loc: Option<String> = match location {
        Some("") => None,
        Some(l) => Some(l.to_string()),
        None => exam.location.clone(),
    };
    let final_completed = is_completed.unwrap_or(exam.is_completed);

    sqlx::query(
        "UPDATE exams SET name = ?, exam_date = ?, start_time = ?, end_time = ?, location = ?, is_completed = ? WHERE id = ?",
    )
    .bind(final_name)
    .bind(final_date)
    .bind(final_start)
    .bind(final_end)
    .bind(final_loc)
    .bind(final_completed)
    .bind(id)
    .execute(pool)
    .await
    .context("更新考试失败")?;

    let exam = find_exam_by_id(pool, id)
        .await?
        .context("查询更新后考试失败")?;
    Ok(exam)
}

pub async fn delete_exam(pool: &MySqlPool, id: i32) -> Result<()> {
    sqlx::query("DELETE FROM exams WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("删除考试失败")?;
    Ok(())
}

pub async fn count_exams_by_user(pool: &MySqlPool, user_id: i32) -> Result<(i64, i64)> {
    let row = sqlx::query(
        "SELECT COUNT(*) as total, COALESCE(SUM(CASE WHEN is_completed = 1 THEN 1 ELSE 0 END), 0) as completed FROM exams WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .context("统计考试数量失败")?;
    let total: Option<i64> = row.try_get("total").unwrap_or(None);
    let completed: Option<i64> = row.try_get("completed").unwrap_or(None);
    Ok((total.unwrap_or(0), completed.unwrap_or(0)))
}

pub async fn find_upcoming_exam(pool: &MySqlPool, user_id: i32) -> Result<Option<Exam>> {
    let today = chrono::Local::now().naive_local().date();
    let exam = sqlx::query_as::<_, Exam>(
        "SELECT id, user_id, name, exam_date, start_time, end_time, location, is_completed, created_at, updated_at FROM exams WHERE user_id = ? AND is_completed = 0 AND exam_date >= ? ORDER BY exam_date ASC LIMIT 1",
    )
    .bind(user_id)
    .bind(today)
    .fetch_optional(pool)
    .await
    .context("查询最近考试失败")?;
    Ok(exam)
}

// ============ 任务模型 ============

#[derive(sqlx::FromRow)]
pub struct Task {
    pub id: i32,
    pub user_id: i32,
    pub task_name: String,
    pub plan_date: chrono::NaiveDate,
    pub is_completed: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

// ============ 任务查询 ============

pub async fn find_today_tasks(pool: &MySqlPool, user_id: i32) -> Result<Vec<Task>> {
    let today = chrono::Local::now().naive_local().date();
    let tasks = sqlx::query_as::<_, Task>(
        "SELECT id, user_id, task_name, plan_date, is_completed, created_at, updated_at FROM tasks WHERE user_id = ? AND plan_date = ? ORDER BY id ASC",
    )
    .bind(user_id)
    .bind(today)
    .fetch_all(pool)
    .await
    .context("查询今日任务失败")?;
    Ok(tasks)
}

pub async fn find_task_by_id(pool: &MySqlPool, id: i32) -> Result<Option<Task>> {
    let task = sqlx::query_as::<_, Task>(
        "SELECT id, user_id, task_name, plan_date, is_completed, created_at, updated_at FROM tasks WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("查询任务失败")?;
    Ok(task)
}

pub async fn update_task_status(pool: &MySqlPool, id: i32, is_completed: bool) -> Result<Task> {
    sqlx::query("UPDATE tasks SET is_completed = ? WHERE id = ?")
        .bind(is_completed)
        .bind(id)
        .execute(pool)
        .await
        .context("更新任务状态失败")?;

    let task = find_task_by_id(pool, id)
        .await?
        .context("查询更新后任务失败")?;
    Ok(task)
}

// ============ 资源模型 ============

#[derive(sqlx::FromRow)]
pub struct Resource {
    pub id: i32,
    pub title: String,
    pub type_tag: String,
    pub school_name: Option<String>,
    pub major_id: Option<i32>,
    pub description: Option<String>,
    pub file_url: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

// ============ 资源查询 ============

pub async fn find_recommended_resources(
    pool: &MySqlPool,
    school_name: &str,
    major_id: i32,
    limit: i64,
) -> Result<Vec<Resource>> {
    let resources = sqlx::query_as::<_, Resource>(
        "SELECT id, title, type_tag, school_name, major_id, description, file_url, created_at, updated_at FROM resources WHERE (school_name = ? AND major_id = ?) OR (school_name = ? AND major_id IS NULL) OR (school_name IS NULL AND major_id = ?) ORDER BY CASE WHEN school_name = ? AND major_id = ? THEN 0 WHEN school_name = ? AND major_id IS NULL THEN 1 WHEN school_name IS NULL AND major_id = ? THEN 2 ELSE 3 END, created_at DESC LIMIT ?",
    )
    .bind(school_name)
    .bind(major_id)
    .bind(school_name)
    .bind(major_id)
    .bind(school_name)
    .bind(major_id)
    .bind(school_name)
    .bind(major_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("查询推荐资源失败")?;
    Ok(resources)
}
