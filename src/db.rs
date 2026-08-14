use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveTime};
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

pub async fn create_task(
    pool: &MySqlPool,
    user_id: i32,
    task_name: &str,
    plan_date: chrono::NaiveDate,
    is_completed: bool,
) -> Result<Task> {
    let result = sqlx::query(
        "INSERT INTO tasks (user_id, task_name, plan_date, is_completed) VALUES (?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(task_name)
    .bind(plan_date)
    .bind(is_completed)
    .execute(pool)
    .await
    .context("插入任务失败")?;

    let task_id = result.last_insert_id() as i32;

    let task = find_task_by_id(pool, task_id)
        .await?
        .context("查询新创建任务失败")?;
    Ok(task)
}

pub async fn update_task_fields(
    pool: &MySqlPool,
    id: i32,
    task_name: Option<&str>,
    plan_date: Option<chrono::NaiveDate>,
    is_completed: Option<bool>,
) -> Result<Task> {
    let task = find_task_by_id(pool, id)
        .await?
        .context("任务不存在")?;

    let final_name = task_name.unwrap_or(&task.task_name);
    let final_date = plan_date.unwrap_or(task.plan_date);
    let final_completed = is_completed.unwrap_or(task.is_completed);

    sqlx::query(
        "UPDATE tasks SET task_name = ?, plan_date = ?, is_completed = ? WHERE id = ?",
    )
    .bind(final_name)
    .bind(final_date)
    .bind(final_completed)
    .bind(id)
    .execute(pool)
    .await
    .context("更新任务失败")?;

    let task = find_task_by_id(pool, id)
        .await?
        .context("查询更新后任务失败")?;
    Ok(task)
}

pub async fn delete_task(pool: &MySqlPool, id: i32) -> Result<()> {
    sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("删除任务失败")?;
    Ok(())
}

// ============ 资源模型 ============

#[derive(sqlx::FromRow)]
pub struct Resource {
    pub id: i32,
    pub title: String,
    pub category: String,
    pub type_tag: String,
    pub school_name: Option<String>,
    pub major_id: Option<i32>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub file_url: String,
    pub view_count: i64,
    pub is_hot: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// 资源列表项（带当前用户收藏状态）
#[derive(sqlx::FromRow)]
pub struct ResourceWithFavorite {
    pub id: i32,
    pub title: String,
    pub category: String,
    pub type_tag: String,
    pub author: Option<String>,
    pub file_url: String,
    pub view_count: i64,
    pub is_hot: bool,
    pub created_at: chrono::NaiveDateTime,
    pub is_favorited: i64,
}

// ============ 资源查询 ============

pub async fn find_recommended_resources(
    pool: &MySqlPool,
    school_name: &str,
    major_id: i32,
    limit: i64,
) -> Result<Vec<Resource>> {
    let resources = sqlx::query_as::<_, Resource>(
        "SELECT id, title, category, type_tag, school_name, major_id, author, description, file_url, view_count, is_hot, created_at, updated_at FROM resources WHERE (school_name = ? AND major_id = ?) OR (school_name = ? AND major_id IS NULL) OR (school_name IS NULL AND major_id = ?) ORDER BY CASE WHEN school_name = ? AND major_id = ? THEN 0 WHEN school_name = ? AND major_id IS NULL THEN 1 WHEN school_name IS NULL AND major_id = ? THEN 2 ELSE 3 END, created_at DESC LIMIT ?",
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

/// 资源是否存在
pub async fn resource_exists(pool: &MySqlPool, id: i32) -> Result<bool> {
    let row = sqlx::query("SELECT 1 FROM resources WHERE id = ? LIMIT 1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("查询资源是否存在失败")?;
    Ok(row.is_some())
}

/// 按分类查询资源列表（带收藏状态），category 为 None 返回全部
pub async fn find_resources_list(
    pool: &MySqlPool,
    user_id: i32,
    school_name: &str,
    major_id: i32,
    category: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ResourceWithFavorite>> {
    let resources = sqlx::query_as::<_, ResourceWithFavorite>(
        "SELECT r.id, r.title, r.category, r.type_tag, r.author, r.file_url, r.view_count, r.is_hot, r.created_at, \
        CASE WHEN f.id IS NOT NULL THEN 1 ELSE 0 END AS is_favorited \
        FROM resources r \
        LEFT JOIN favorites f ON f.resource_id = r.id AND f.user_id = ? \
        WHERE (r.school_name = ? OR r.school_name IS NULL) \
        AND (r.major_id = ? OR r.major_id IS NULL) \
        AND (? IS NULL OR r.category = ?) \
        ORDER BY r.is_hot DESC, r.created_at DESC \
        LIMIT ? OFFSET ?",
    )
    .bind(user_id)
    .bind(school_name)
    .bind(major_id)
    .bind(category)
    .bind(category)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("查询资源列表失败")?;
    Ok(resources)
}

/// 统计符合条件的资源总数，category 为 None 统计全部
pub async fn count_resources_list(
    pool: &MySqlPool,
    school_name: &str,
    major_id: i32,
    category: Option<&str>,
) -> Result<i64> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM resources r \
        WHERE (r.school_name = ? OR r.school_name IS NULL) \
        AND (r.major_id = ? OR r.major_id IS NULL) \
        AND (? IS NULL OR r.category = ?)",
    )
    .bind(school_name)
    .bind(major_id)
    .bind(category)
    .bind(category)
    .fetch_one(pool)
    .await
    .context("统计资源总数失败")?;
    let total: Option<i64> = row.try_get("cnt").unwrap_or(None);
    Ok(total.unwrap_or(0))
}

/// 关键词搜索资源（匹配 title 和 description），仅返回与用户学校相关的资源
pub async fn search_resources(
    pool: &MySqlPool,
    user_id: i32,
    school_name: &str,
    major_id: i32,
    keyword: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<ResourceWithFavorite>> {
    let pattern = format!("%{}%", keyword);
    let resources = sqlx::query_as::<_, ResourceWithFavorite>(
        "SELECT r.id, r.title, r.category, r.type_tag, r.author, r.file_url, r.view_count, r.is_hot, r.created_at, \
        CASE WHEN f.id IS NOT NULL THEN 1 ELSE 0 END AS is_favorited \
        FROM resources r \
        LEFT JOIN favorites f ON f.resource_id = r.id AND f.user_id = ? \
        WHERE (r.school_name = ? OR r.school_name IS NULL) \
        AND (r.major_id = ? OR r.major_id IS NULL) \
        AND (r.title LIKE ? OR r.description LIKE ?) \
        ORDER BY r.is_hot DESC, r.created_at DESC \
        LIMIT ? OFFSET ?",
    )
    .bind(user_id)
    .bind(school_name)
    .bind(major_id)
    .bind(&pattern)
    .bind(&pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("搜索资源失败")?;
    Ok(resources)
}

/// 统计搜索结果总数
pub async fn count_search_resources(
    pool: &MySqlPool,
    school_name: &str,
    major_id: i32,
    keyword: &str,
) -> Result<i64> {
    let pattern = format!("%{}%", keyword);
    let row = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM resources r \
        WHERE (r.school_name = ? OR r.school_name IS NULL) \
        AND (r.major_id = ? OR r.major_id IS NULL) \
        AND (r.title LIKE ? OR r.description LIKE ?)",
    )
    .bind(school_name)
    .bind(major_id)
    .bind(&pattern)
    .bind(&pattern)
    .fetch_one(pool)
    .await
    .context("统计搜索结果失败")?;
    let total: Option<i64> = row.try_get("cnt").unwrap_or(None);
    Ok(total.unwrap_or(0))
}

/// 查询热门资源（取一条用于推荐）
pub async fn find_hot_resource(
    pool: &MySqlPool,
    school_name: &str,
    major_id: i32,
) -> Result<Option<Resource>> {
    let resource = sqlx::query_as::<_, Resource>(
        "SELECT id, title, category, type_tag, school_name, major_id, author, description, file_url, view_count, is_hot, created_at, updated_at FROM resources \
        WHERE is_hot = 1 AND (school_name = ? OR school_name IS NULL) AND (major_id = ? OR major_id IS NULL) \
        ORDER BY view_count DESC LIMIT 1",
    )
    .bind(school_name)
    .bind(major_id)
    .fetch_optional(pool)
    .await
    .context("查询热门资源失败")?;
    Ok(resource)
}

// ============ 收藏查询 ============

/// 查询用户是否已收藏某资源
pub async fn find_favorite(pool: &MySqlPool, user_id: i32, resource_id: i32) -> Result<bool> {
    let row = sqlx::query(
        "SELECT 1 FROM favorites WHERE user_id = ? AND resource_id = ? LIMIT 1",
    )
    .bind(user_id)
    .bind(resource_id)
    .fetch_optional(pool)
    .await
    .context("查询收藏记录失败")?;
    Ok(row.is_some())
}

/// 添加收藏
pub async fn create_favorite(pool: &MySqlPool, user_id: i32, resource_id: i32) -> Result<()> {
    sqlx::query("INSERT IGNORE INTO favorites (user_id, resource_id) VALUES (?, ?)")
        .bind(user_id)
        .bind(resource_id)
        .execute(pool)
        .await
        .context("添加收藏失败")?;
    Ok(())
}

/// 取消收藏
pub async fn delete_favorite(pool: &MySqlPool, user_id: i32, resource_id: i32) -> Result<()> {
    sqlx::query("DELETE FROM favorites WHERE user_id = ? AND resource_id = ?")
        .bind(user_id)
        .bind(resource_id)
        .execute(pool)
        .await
        .context("取消收藏失败")?;
    Ok(())
}

// ============ AI 对话记录 ============

/// 存储一条 AI 对话消息
pub async fn create_ai_message(
    pool: &MySqlPool,
    user_id: i32,
    conversation_id: &str,
    role: &str,
    content: &str,
    attachment_url: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO ai_conversations (user_id, conversation_id, role, content, attachment_url) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(conversation_id)
    .bind(role)
    .bind(content)
    .bind(attachment_url)
    .execute(pool)
    .await
    .context("存储 AI 对话记录失败")?;
    Ok(())
}

// ============ 通用工具 ============

/// 判断 sqlx 错误是否为唯一约束冲突（MySQL Duplicate entry）
pub fn is_duplicate_key(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db) => db.message().contains("Duplicate entry"),
        _ => false,
    }
}

/// 判断 anyhow 包装的错误是否为唯一约束冲突（MySQL Duplicate entry）
pub fn is_duplicate_key_anyhow(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<sqlx::Error>()
            .map(is_duplicate_key)
            .unwrap_or(false)
    })
}

// ============ 打卡模型 ============

#[derive(sqlx::FromRow)]
pub struct CheckinRecord {
    pub id: i32,
    pub user_id: i32,
    pub checkin_date: chrono::NaiveDate,
    pub subjects: Option<String>,
    pub duration: Option<String>,
    pub note: Option<String>,
    pub tags: Option<String>,
    pub continuous_days: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// 同校打卡统计行（用于排行与个人排名计算）
#[derive(sqlx::FromRow)]
pub struct CheckinStatRow {
    pub user_id: i32,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub continuous_days: i32,
    pub total_days: i64,
}

// ============ 打卡查询 ============

/// 查询用户某天的打卡记录
pub async fn find_checkin_by_user_date(
    pool: &MySqlPool,
    user_id: i32,
    date: chrono::NaiveDate,
) -> Result<Option<CheckinRecord>> {
    let rec = sqlx::query_as::<_, CheckinRecord>(
        "SELECT id, user_id, checkin_date, CAST(subjects AS CHAR) AS subjects, duration, note, CAST(tags AS CHAR) AS tags, continuous_days, created_at, updated_at FROM checkin_records WHERE user_id = ? AND checkin_date = ?",
    )
    .bind(user_id)
    .bind(date)
    .fetch_optional(pool)
    .await
    .context("查询打卡记录失败")?;
    Ok(rec)
}

/// 查询用户最近一次打卡记录
pub async fn find_latest_checkin_by_user(
    pool: &MySqlPool,
    user_id: i32,
) -> Result<Option<CheckinRecord>> {
    let rec = sqlx::query_as::<_, CheckinRecord>(
        "SELECT id, user_id, checkin_date, CAST(subjects AS CHAR) AS subjects, duration, note, CAST(tags AS CHAR) AS tags, continuous_days, created_at, updated_at FROM checkin_records WHERE user_id = ? ORDER BY checkin_date DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .context("查询最近打卡记录失败")?;
    Ok(rec)
}

/// 查询用户某月的所有打卡日期
pub async fn find_checkin_dates_by_user_month(
    pool: &MySqlPool,
    user_id: i32,
    year: i32,
    month: u32,
) -> Result<Vec<chrono::NaiveDate>> {
    let rows = sqlx::query("SELECT checkin_date FROM checkin_records WHERE user_id = ? AND YEAR(checkin_date) = ? AND MONTH(checkin_date) = ?")
        .bind(user_id)
        .bind(year)
        .bind(month)
        .fetch_all(pool)
        .await
        .context("查询月度打卡日期失败")?;
    let mut dates = Vec::new();
    for row in rows {
        let d: chrono::NaiveDate = row
            .try_get("checkin_date")
            .context("解析打卡日期失败")?;
        dates.push(d);
    }
    Ok(dates)
}

/// 统计用户累计打卡天数
pub async fn count_checkins_by_user(pool: &MySqlPool, user_id: i32) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM checkin_records WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .context("统计打卡天数失败")?;
    let total = row
        .try_get::<i64, _>("cnt")
        .ok()
        .or_else(|| row.try_get::<Option<i64>, _>("cnt").ok().flatten())
        .unwrap_or(0);
    Ok(total)
}

/// 创建打卡记录，自动计算连续天数（内部查询昨日记录）
pub async fn create_checkin(
    pool: &MySqlPool,
    user_id: i32,
    date: chrono::NaiveDate,
    subjects: Option<&str>,
    duration: Option<&str>,
    note: Option<&str>,
    tags: Option<&str>,
) -> Result<CheckinRecord> {
    // 计算连续天数：若昨日已打卡，则昨日连续天数 + 1，否则为 1
    let yesterday = date - chrono::Duration::days(1);
    let continuous_days = match find_checkin_by_user_date(pool, user_id, yesterday).await? {
        Some(rec) => rec.continuous_days + 1,
        None => 1,
    };

    let result = sqlx::query(
        "INSERT INTO checkin_records (user_id, checkin_date, subjects, duration, note, tags, continuous_days) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(date)
    .bind(subjects)
    .bind(duration)
    .bind(note)
    .bind(tags)
    .bind(continuous_days)
    .execute(pool)
    .await
    .context("插入打卡记录失败")?;

    let id = result.last_insert_id() as i32;
    let rec = find_checkin_by_user_date(pool, user_id, date)
        .await?
        .context("查询新创建打卡记录失败")?;
    let _ = id;
    Ok(rec)
}

/// 查询同校所有用户的打卡统计（按连续天数、总天数降序）
pub async fn find_school_checkin_stats(
    pool: &MySqlPool,
    school_name: &str,
) -> Result<Vec<CheckinStatRow>> {
    let rows = sqlx::query_as::<_, CheckinStatRow>(
        "SELECT u.id AS user_id, u.nickname, u.avatar_url, \
        CASE WHEN latest.checkin_date >= DATE_SUB(CURDATE(), INTERVAL 1 DAY) THEN latest.continuous_days ELSE 0 END AS continuous_days, \
        COALESCE(cnt.total_days, 0) AS total_days \
        FROM users u \
        LEFT JOIN checkin_records latest ON latest.user_id = u.id AND latest.id = (SELECT id FROM checkin_records WHERE user_id = u.id ORDER BY checkin_date DESC LIMIT 1) \
        LEFT JOIN (SELECT user_id, COUNT(*) AS total_days FROM checkin_records GROUP BY user_id) cnt ON cnt.user_id = u.id \
        WHERE u.school_name = ? \
        ORDER BY continuous_days DESC, total_days DESC, u.id ASC",
    )
    .bind(school_name)
    .fetch_all(pool)
    .await
    .context("查询同校打卡统计失败")?;
    Ok(rows)
}

// ============ 小队模型 ============

#[derive(sqlx::FromRow)]
pub struct Team {
    pub id: i32,
    pub name: String,
    pub subject: String,
    pub description: Option<String>,
    pub creator_id: i32,
    pub member_count: i32,
    pub max_members: i32,
    pub need_approval: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// 小队列表行（含统计字段与当前用户相关状态）
#[derive(sqlx::FromRow)]
pub struct TeamListRow {
    pub id: i32,
    pub name: String,
    pub subject: String,
    pub description: Option<String>,
    pub creator_id: i32,
    pub max_members: i32,
    pub need_approval: bool,
    pub member_count: i64,
    pub online_count: i64,
    pub total_checkins: i64,
    pub pending_requests: i64,
    pub join_status: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

/// 小队详情行（含统计字段与当前用户是否为成员）
#[derive(sqlx::FromRow)]
pub struct TeamDetailRow {
    pub id: i32,
    pub name: String,
    pub subject: String,
    pub description: Option<String>,
    pub creator_id: i32,
    pub max_members: i32,
    pub need_approval: bool,
    pub member_count: i64,
    pub online_count: i64,
    pub total_checkins: i64,
    pub is_member: i64,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(sqlx::FromRow)]
pub struct TeamMemberInfo {
    pub user_id: i32,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub joined_at: chrono::NaiveDateTime,
}

#[derive(sqlx::FromRow)]
pub struct TeamJoinRequest {
    pub id: i32,
    pub team_id: i32,
    pub user_id: i32,
    pub status: String,
    pub applied_at: chrono::NaiveDateTime,
    pub processed_at: Option<chrono::NaiveDateTime>,
}

#[derive(sqlx::FromRow)]
pub struct JoinRequestInfo {
    pub id: i32,
    pub user_id: i32,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub applied_at: chrono::NaiveDateTime,
}

// ============ 小队查询 ============

/// 小队列表查询的公共 SELECT 部分（含统计与当前用户状态）
const TEAM_LIST_SELECT: &str = "t.id, t.name, t.subject, t.description, t.creator_id, t.max_members, t.need_approval, t.created_at, \
(SELECT COUNT(*) FROM team_members WHERE team_id = t.id) AS member_count, \
(SELECT COUNT(DISTINCT tm.user_id) FROM team_members tm JOIN checkin_records cr ON cr.user_id = tm.user_id AND cr.checkin_date = CURDATE() WHERE tm.team_id = t.id) AS online_count, \
(SELECT COUNT(*) FROM checkin_records cr JOIN team_members tm ON tm.user_id = cr.user_id WHERE tm.team_id = t.id) AS total_checkins, \
(SELECT COUNT(*) FROM team_join_requests WHERE team_id = t.id AND status = '待审核') AS pending_requests, \
(SELECT r.status FROM team_join_requests r WHERE r.team_id = t.id AND r.user_id = ? ORDER BY r.applied_at DESC LIMIT 1) AS join_status";

/// 查询用户已加入的小队
pub async fn find_my_teams(
    pool: &MySqlPool,
    user_id: i32,
    subject: Option<&str>,
) -> Result<Vec<TeamListRow>> {
    let pattern = subject.map(|s| format!("%{}%", s));
    let rows = sqlx::query_as::<_, TeamListRow>(&format!(
        "SELECT {} FROM teams t WHERE t.id IN (SELECT team_id FROM team_members WHERE user_id = ?) \
        AND (? IS NULL OR t.subject LIKE ?) ORDER BY t.created_at DESC",
        TEAM_LIST_SELECT
    ))
    .bind(user_id)      // 1. TEAM_LIST_SELECT 内部 r.user_id = ?
    .bind(user_id)      // 2. WHERE user_id = ? (IN 子句)
    .bind(&pattern)     // 3. ? IS NULL
    .bind(&pattern)     // 4. t.subject LIKE ?
    .fetch_all(pool)
    .await
    .context("查询我的小队失败")?;
    Ok(rows)
}

/// 查询推荐小队（用户未加入的）
pub async fn find_recommended_teams(
    pool: &MySqlPool,
    user_id: i32,
    subject: Option<&str>,
) -> Result<Vec<TeamListRow>> {
    let pattern = subject.map(|s| format!("%{}%", s));
    let rows = sqlx::query_as::<_, TeamListRow>(&format!(
        "SELECT {} FROM teams t WHERE t.id NOT IN (SELECT team_id FROM team_members WHERE user_id = ?) \
        AND (? IS NULL OR t.subject LIKE ?) ORDER BY t.created_at DESC",
        TEAM_LIST_SELECT
    ))
    .bind(user_id)      // 1. TEAM_LIST_SELECT 内部 r.user_id = ?
    .bind(user_id)      // 2. NOT IN 子句 user_id = ?
    .bind(&pattern)     // 3. ? IS NULL
    .bind(&pattern)     // 4. t.subject LIKE ?
    .fetch_all(pool)
    .await
    .context("查询推荐小队失败")?;
    Ok(rows)
}

/// 查询热门小队（用于社区主页，取全部后在内存排序取前 N）
pub async fn find_hot_teams(pool: &MySqlPool, user_id: i32) -> Result<Vec<TeamListRow>> {
    let rows = sqlx::query_as::<_, TeamListRow>(&format!(
        "SELECT {} FROM teams t ORDER BY t.created_at DESC",
        TEAM_LIST_SELECT
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("查询热门小队失败")?;
    Ok(rows)
}

/// 查询小队详情（含统计与当前用户是否为成员）
pub async fn find_team_detail(
    pool: &MySqlPool,
    team_id: i32,
    user_id: i32,
) -> Result<Option<TeamDetailRow>> {
    let row = sqlx::query_as::<_, TeamDetailRow>(
        "SELECT t.id, t.name, t.subject, t.description, t.creator_id, t.max_members, t.need_approval, t.created_at, \
        (SELECT COUNT(*) FROM team_members WHERE team_id = t.id) AS member_count, \
        (SELECT COUNT(DISTINCT tm.user_id) FROM team_members tm JOIN checkin_records cr ON cr.user_id = tm.user_id AND cr.checkin_date = CURDATE() WHERE tm.team_id = t.id) AS online_count, \
        (SELECT COUNT(*) FROM checkin_records cr JOIN team_members tm ON tm.user_id = cr.user_id WHERE tm.team_id = t.id) AS total_checkins, \
        (SELECT COUNT(*) FROM team_members WHERE team_id = t.id AND user_id = ?) AS is_member \
        FROM teams t WHERE t.id = ?",
    )
    .bind(user_id)
    .bind(team_id)
    .fetch_optional(pool)
    .await
    .context("查询小队详情失败")?;
    Ok(row)
}

/// 查询小队成员列表
pub async fn find_team_members(pool: &MySqlPool, team_id: i32) -> Result<Vec<TeamMemberInfo>> {
    let rows = sqlx::query_as::<_, TeamMemberInfo>(
        "SELECT tm.user_id, u.nickname, u.avatar_url, tm.role, tm.joined_at \
        FROM team_members tm JOIN users u ON u.id = tm.user_id \
        WHERE tm.team_id = ? ORDER BY CASE WHEN tm.role = '队长' THEN 0 ELSE 1 END, tm.joined_at ASC",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await
    .context("查询小队成员失败")?;
    Ok(rows)
}

/// 查询用户在小队中的成员记录
pub async fn find_team_member(
    pool: &MySqlPool,
    team_id: i32,
    user_id: i32,
) -> Result<Option<TeamMemberInfo>> {
    let row = sqlx::query_as::<_, TeamMemberInfo>(
        "SELECT tm.user_id, u.nickname, u.avatar_url, tm.role, tm.joined_at \
        FROM team_members tm JOIN users u ON u.id = tm.user_id \
        WHERE tm.team_id = ? AND tm.user_id = ?",
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .context("查询小队成员失败")?;
    Ok(row)
}

/// 创建小队（事务：插入小队 + 队长成员记录）
pub async fn create_team(
    pool: &MySqlPool,
    name: &str,
    subject: &str,
    description: Option<&str>,
    need_approval: bool,
    creator_id: i32,
) -> Result<Team> {
    let mut tx = pool.begin().await?;
    let result = sqlx::query(
        "INSERT INTO teams (name, subject, description, creator_id, member_count, max_members, need_approval) VALUES (?, ?, ?, ?, 1, 30, ?)",
    )
    .bind(name)
    .bind(subject)
    .bind(description)
    .bind(creator_id)
    .bind(need_approval)
    .execute(&mut *tx)
    .await
    .context("插入小队失败")?;
    let team_id = result.last_insert_id() as i32;
    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES (?, ?, '队长')")
        .bind(team_id)
        .bind(creator_id)
        .execute(&mut *tx)
        .await
        .context("插入队长成员记录失败")?;
    tx.commit().await?;

    let team = sqlx::query_as::<_, Team>(
        "SELECT id, name, subject, description, creator_id, member_count, max_members, need_approval, created_at, updated_at FROM teams WHERE id = ?",
    )
    .bind(team_id)
    .fetch_one(pool)
    .await
    .context("查询新创建小队失败")?;
    Ok(team)
}

/// 直接加入小队（事务：插入成员 + 增加成员数）
pub async fn join_team_directly(
    pool: &MySqlPool,
    team_id: i32,
    user_id: i32,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES (?, ?, '成员')")
        .bind(team_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE teams SET member_count = member_count + 1 WHERE id = ?")
        .bind(team_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// 查询用户对小队的申请记录
pub async fn find_join_request(
    pool: &MySqlPool,
    team_id: i32,
    user_id: i32,
) -> Result<Option<TeamJoinRequest>> {
    let row = sqlx::query_as::<_, TeamJoinRequest>(
        "SELECT id, team_id, user_id, status, applied_at, processed_at FROM team_join_requests WHERE team_id = ? AND user_id = ?",
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .context("查询入队申请失败")?;
    Ok(row)
}

/// 创建入队申请（待审核）
pub async fn create_join_request(
    pool: &MySqlPool,
    team_id: i32,
    user_id: i32,
) -> Result<TeamJoinRequest> {
    sqlx::query("INSERT INTO team_join_requests (team_id, user_id, status) VALUES (?, ?, '待审核')")
        .bind(team_id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("创建入队申请失败")?;
    let req = find_join_request(pool, team_id, user_id)
        .await?
        .context("查询新创建申请失败")?;
    Ok(req)
}

/// 将已拒绝的申请重置为待审核（重新申请）
pub async fn reset_join_request_to_pending(
    pool: &MySqlPool,
    team_id: i32,
    user_id: i32,
) -> Result<TeamJoinRequest> {
    sqlx::query("UPDATE team_join_requests SET status = '待审核', applied_at = NOW(), processed_at = NULL WHERE team_id = ? AND user_id = ?")
        .bind(team_id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("重置入队申请失败")?;
    let req = find_join_request(pool, team_id, user_id)
        .await?
        .context("查询重置后申请失败")?;
    Ok(req)
}

/// 查询小队的待审核申请列表
pub async fn find_pending_join_requests(
    pool: &MySqlPool,
    team_id: i32,
) -> Result<Vec<JoinRequestInfo>> {
    let rows = sqlx::query_as::<_, JoinRequestInfo>(
        "SELECT r.id, r.user_id, u.nickname, u.avatar_url, r.applied_at \
        FROM team_join_requests r JOIN users u ON u.id = r.user_id \
        WHERE r.team_id = ? AND r.status = '待审核' ORDER BY r.applied_at ASC",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await
    .context("查询待审核申请失败")?;
    Ok(rows)
}

/// 根据 ID 查询申请记录
pub async fn find_join_request_by_id(
    pool: &MySqlPool,
    application_id: i32,
) -> Result<Option<TeamJoinRequest>> {
    let row = sqlx::query_as::<_, TeamJoinRequest>(
        "SELECT id, team_id, user_id, status, applied_at, processed_at FROM team_join_requests WHERE id = ?",
    )
    .bind(application_id)
    .fetch_optional(pool)
    .await
    .context("查询申请记录失败")?;
    Ok(row)
}

/// 通过申请（事务：更新申请状态 + 插入成员 + 增加成员数）
pub async fn approve_join_request(
    pool: &MySqlPool,
    application_id: i32,
    team_id: i32,
    user_id: i32,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE team_join_requests SET status = '已通过', processed_at = NOW() WHERE id = ?")
        .bind(application_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES (?, ?, '成员')")
        .bind(team_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE teams SET member_count = member_count + 1 WHERE id = ?")
        .bind(team_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// 拒绝申请
pub async fn reject_join_request(
    pool: &MySqlPool,
    application_id: i32,
) -> Result<()> {
    sqlx::query("UPDATE team_join_requests SET status = '已拒绝', processed_at = NOW() WHERE id = ?")
        .bind(application_id)
        .execute(pool)
        .await
        .context("拒绝入队申请失败")?;
    Ok(())
}

/// 退出小队（事务：删除成员 + 减少成员数）
pub async fn leave_team(pool: &MySqlPool, team_id: i32, user_id: i32) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM team_members WHERE team_id = ? AND user_id = ?")
        .bind(team_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE teams SET member_count = GREATEST(member_count - 1, 0) WHERE id = ?")
        .bind(team_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// 解散小队（事务：删除成员 + 申请 + 小队）
pub async fn dissolve_team(pool: &MySqlPool, team_id: i32) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM team_members WHERE team_id = ?")
        .bind(team_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM team_join_requests WHERE team_id = ?")
        .bind(team_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM teams WHERE id = ?")
        .bind(team_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

// ============ 投票模型 ============

#[derive(sqlx::FromRow)]
pub struct Vote {
    pub id: i32,
    pub subject: String,
    pub title: String,
    pub description: Option<String>,
    pub vote_count: i32,
    pub status: String,
    pub submitter_id: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// 投票列表项（含当前用户是否已投票）
#[derive(sqlx::FromRow)]
pub struct VoteListItem {
    pub id: i32,
    pub subject: String,
    pub title: String,
    pub description: Option<String>,
    pub vote_count: i32,
    pub has_voted: i64,
}

/// 我的投票记录行
#[derive(sqlx::FromRow)]
pub struct MyVoteRecord {
    pub vote_id: i32,
    pub subject: String,
    pub title: String,
    pub vote_count: i32,
    pub voted_at: chrono::NaiveDateTime,
}

// ============ 投票查询 ============

/// 查询已通过的考点列表（按票数降序，可按科目筛选）
pub async fn find_passed_votes(
    pool: &MySqlPool,
    user_id: i32,
    subject: Option<&str>,
) -> Result<Vec<VoteListItem>> {
    let pattern = subject.map(|s| format!("%{}%", s));
    let rows = sqlx::query_as::<_, VoteListItem>(
        "SELECT v.id, v.subject, v.title, v.description, v.vote_count, \
        CASE WHEN vr.user_id IS NOT NULL THEN 1 ELSE 0 END AS has_voted \
        FROM votes v LEFT JOIN vote_records vr ON vr.vote_id = v.id AND vr.user_id = ? \
        WHERE v.status = '已通过' AND (? IS NULL OR v.subject LIKE ?) \
        ORDER BY v.vote_count DESC, v.created_at DESC",
    )
    .bind(user_id)
    .bind(&pattern)
    .bind(&pattern)
    .fetch_all(pool)
    .await
    .context("查询投票列表失败")?;
    Ok(rows)
}

/// 统计已通过考点的总票数（用于置信度计算）
pub async fn sum_passed_votes_count(pool: &MySqlPool) -> Result<i64> {
    let row = sqlx::query("SELECT COALESCE(SUM(vote_count), 0) AS total FROM votes WHERE status = '已通过'")
        .fetch_one(pool)
        .await
        .context("统计总票数失败")?;
    // sqlx 对 COALESCE(SUM(...)) 可能返回 Decimal / i64 / Option<i64>，依次尝试
    let total = row
        .try_get::<i64, _>("total")
        .ok()
        .or_else(|| row.try_get::<Option<i64>, _>("total").ok().flatten())
        .unwrap_or(0);
    Ok(total)
}

/// 根据 ID 查询考点
pub async fn find_vote_by_id(pool: &MySqlPool, id: i32) -> Result<Option<Vote>> {
    let row = sqlx::query_as::<_, Vote>(
        "SELECT id, subject, title, description, vote_count, status, submitter_id, created_at, updated_at FROM votes WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("查询考点失败")?;
    Ok(row)
}

/// 提交新考点（待审核）
pub async fn create_vote(
    pool: &MySqlPool,
    subject: &str,
    title: &str,
    description: Option<&str>,
    submitter_id: i32,
) -> Result<Vote> {
    let result = sqlx::query(
        "INSERT INTO votes (subject, title, description, vote_count, status, submitter_id) VALUES (?, ?, ?, 0, '待审核', ?)",
    )
    .bind(subject)
    .bind(title)
    .bind(description)
    .bind(submitter_id)
    .execute(pool)
    .await
    .context("插入考点失败")?;
    let id = result.last_insert_id() as i32;
    let vote = find_vote_by_id(pool, id)
        .await?
        .context("查询新创建考点失败")?;
    Ok(vote)
}

/// 查询用户是否已对某考点投票
pub async fn has_voted(pool: &MySqlPool, vote_id: i32, user_id: i32) -> Result<bool> {
    let row = sqlx::query("SELECT 1 FROM vote_records WHERE vote_id = ? AND user_id = ? LIMIT 1")
        .bind(vote_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .context("查询投票记录失败")?;
    Ok(row.is_some())
}

/// 投票（事务：插入投票记录 + 增加票数）
pub async fn cast_vote(
    pool: &MySqlPool,
    vote_id: i32,
    user_id: i32,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO vote_records (vote_id, user_id) VALUES (?, ?)")
        .bind(vote_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE votes SET vote_count = vote_count + 1 WHERE id = ?")
        .bind(vote_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// 查询我的投票记录
pub async fn find_my_votes(pool: &MySqlPool, user_id: i32) -> Result<Vec<MyVoteRecord>> {
    let rows = sqlx::query_as::<_, MyVoteRecord>(
        "SELECT v.id AS vote_id, v.subject, v.title, v.vote_count, vr.created_at AS voted_at \
        FROM vote_records vr JOIN votes v ON v.id = vr.vote_id \
        WHERE vr.user_id = ? ORDER BY vr.created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("查询我的投票记录失败")?;
    Ok(rows)
}

/// 查询我提交的考点
pub async fn find_my_submissions(pool: &MySqlPool, user_id: i32) -> Result<Vec<Vote>> {
    let rows = sqlx::query_as::<_, Vote>(
        "SELECT id, subject, title, description, vote_count, status, submitter_id, created_at, updated_at FROM votes WHERE submitter_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("查询我提交的考点失败")?;
    Ok(rows)
}

// ============ 资源上传/我的上传 ============

/// 上传记录行
#[derive(sqlx::FromRow)]
pub struct UploadedResource {
    pub id: i32,
    pub title: String,
    pub category: String,
    pub status: String,
    pub view_count: i64,
    pub reject_reason: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

/// 社区主页最新资料行
#[derive(sqlx::FromRow)]
pub struct LatestResourceRow {
    pub id: i32,
    pub title: String,
    pub author: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

/// 创建共享资料（审核中）
pub async fn create_resource(
    pool: &MySqlPool,
    uploader_id: i32,
    title: &str,
    subject: Option<&str>,
    category: &str,
    type_tag: &str,
    school_name: Option<&str>,
    author: Option<&str>,
    description: Option<&str>,
    file_url: &str,
) -> Result<i32> {
    let result = sqlx::query(
        "INSERT INTO resources (title, category, type_tag, school_name, major_id, author, uploader_id, subject, description, file_url, view_count, is_hot, status) \
        VALUES (?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, 0, 0, '审核中')",
    )
    .bind(title)
    .bind(category)
    .bind(type_tag)
    .bind(school_name)
    .bind(author)
    .bind(uploader_id)
    .bind(subject)
    .bind(description)
    .bind(file_url)
    .execute(pool)
    .await
    .context("插入资源失败")?;
    Ok(result.last_insert_id() as i32)
}

/// 查询某资源的创建时间（用于上传成功响应）
pub async fn find_resource_created_at(pool: &MySqlPool, id: i32) -> Result<Option<chrono::NaiveDateTime>> {
    let row = sqlx::query("SELECT created_at FROM resources WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("查询资源创建时间失败")?;
    if let Some(r) = row {
        let created_at: chrono::NaiveDateTime = r
            .try_get("created_at")
            .context("解析资源创建时间失败")?;
        return Ok(Some(created_at));
    }
    Ok(None)
}

/// 查询我的上传记录
pub async fn find_my_uploads(pool: &MySqlPool, uploader_id: i32) -> Result<Vec<UploadedResource>> {
    let rows = sqlx::query_as::<_, UploadedResource>(
        "SELECT id, title, category, status, view_count, reject_reason, created_at FROM resources WHERE uploader_id = ? ORDER BY created_at DESC",
    )
    .bind(uploader_id)
    .fetch_all(pool)
    .await
    .context("查询我的上传记录失败")?;
    Ok(rows)
}

/// 查询本校最新已上线资料（社区主页）
pub async fn find_latest_resources_by_school(
    pool: &MySqlPool,
    school_name: &str,
    limit: i64,
) -> Result<Vec<LatestResourceRow>> {
    let rows = sqlx::query_as::<_, LatestResourceRow>(
        "SELECT id, title, author, created_at FROM resources WHERE school_name = ? AND status = '已上线' ORDER BY created_at DESC LIMIT ?",
    )
    .bind(school_name)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("查询最新资料失败")?;
    Ok(rows)
}

// ============ 积分 ============

/// 增加用户积分（不存在则创建）
pub async fn add_user_points(pool: &MySqlPool, user_id: i32, points: i32) -> Result<()> {
    sqlx::query(
        "INSERT INTO user_points (user_id, total_points) VALUES (?, ?) \
        ON DUPLICATE KEY UPDATE total_points = total_points + VALUES(total_points)",
    )
    .bind(user_id)
    .bind(points)
    .execute(pool)
    .await
    .context("增加积分失败")?;
    Ok(())
}

// ============ 工具模块：每日一问 ============

#[derive(sqlx::FromRow)]
pub struct DailyQuestion {
    pub id: i32,
    pub subject: String,
    pub question: String,
    pub options: Option<String>,
    pub answer: String,
    pub explanation: Option<String>,
    pub difficulty: i32,
    pub date: chrono::NaiveDate,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(sqlx::FromRow)]
pub struct QuestionRecord {
    pub id: i32,
    pub user_id: i32,
    pub question_id: i32,
    pub answered_at: chrono::NaiveDateTime,
    pub selected: String,
    pub is_correct: bool,
}

#[derive(sqlx::FromRow)]
pub struct QuestionHistoryRow {
    pub id: i32,
    pub question_id: i32,
    pub subject: String,
    pub selected: String,
    pub is_correct: bool,
    pub answered_at: chrono::NaiveDateTime,
}

pub async fn find_daily_question_by_id(
    pool: &MySqlPool,
    id: i32,
) -> Result<Option<DailyQuestion>> {
    let row = sqlx::query_as::<_, DailyQuestion>(
        "SELECT id, subject, question, CAST(options AS CHAR) AS options, answer, explanation, difficulty, date, created_at \
        FROM daily_questions WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("查询每日一题失败")?;
    Ok(row)
}

pub async fn find_daily_question_by_date(
    pool: &MySqlPool,
    date: chrono::NaiveDate,
) -> Result<Option<DailyQuestion>> {
    let row = sqlx::query_as::<_, DailyQuestion>(
        "SELECT id, subject, question, CAST(options AS CHAR) AS options, answer, explanation, difficulty, date, created_at \
        FROM daily_questions WHERE date = ?",
    )
    .bind(date)
    .fetch_optional(pool)
    .await
    .context("查询今日每日一题失败")?;
    Ok(row)
}

pub async fn find_question_record(
    pool: &MySqlPool,
    user_id: i32,
    question_id: i32,
) -> Result<Option<QuestionRecord>> {
    let row = sqlx::query_as::<_, QuestionRecord>(
        "SELECT id, user_id, question_id, answered_at, selected, is_correct \
        FROM question_records WHERE user_id = ? AND question_id = ?",
    )
    .bind(user_id)
    .bind(question_id)
    .fetch_optional(pool)
    .await
    .context("查询答题记录失败")?;
    Ok(row)
}

pub async fn find_question_record_by_id(
    pool: &MySqlPool,
    id: i32,
) -> Result<Option<QuestionRecord>> {
    let row = sqlx::query_as::<_, QuestionRecord>(
        "SELECT id, user_id, question_id, answered_at, selected, is_correct \
        FROM question_records WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("查询答题记录失败")?;
    Ok(row)
}

pub async fn create_question_record(
    pool: &MySqlPool,
    user_id: i32,
    question_id: i32,
    selected: &str,
    is_correct: bool,
) -> Result<QuestionRecord> {
    let result = sqlx::query(
        "INSERT INTO question_records (user_id, question_id, selected, is_correct) VALUES (?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(question_id)
    .bind(selected)
    .bind(is_correct)
    .execute(pool)
    .await
    .context("插入答题记录失败")?;
    let id = result.last_insert_id() as i32;
    let record = find_question_record_by_id(pool, id)
        .await?
        .context("查询新建答题记录失败")?;
    Ok(record)
}

pub async fn find_question_records_by_month(
    pool: &MySqlPool,
    user_id: i32,
    start: chrono::NaiveDateTime,
    end: chrono::NaiveDateTime,
    only_wrong: bool,
) -> Result<Vec<QuestionHistoryRow>> {
    let rows = if only_wrong {
        sqlx::query_as::<_, QuestionHistoryRow>(
            "SELECT qr.id, qr.question_id, dq.subject, qr.selected, qr.is_correct, qr.answered_at \
            FROM question_records qr JOIN daily_questions dq ON dq.id = qr.question_id \
            WHERE qr.user_id = ? AND qr.answered_at >= ? AND qr.answered_at < ? AND qr.is_correct = 0 \
            ORDER BY qr.answered_at DESC",
        )
    } else {
        sqlx::query_as::<_, QuestionHistoryRow>(
            "SELECT qr.id, qr.question_id, dq.subject, qr.selected, qr.is_correct, qr.answered_at \
            FROM question_records qr JOIN daily_questions dq ON dq.id = qr.question_id \
            WHERE qr.user_id = ? AND qr.answered_at >= ? AND qr.answered_at < ? \
            ORDER BY qr.answered_at DESC",
        )
    };
    let rows = rows
        .bind(user_id)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
        .context("查询答题历史失败")?;
    Ok(rows)
}

pub async fn find_question_dates_by_month(
    pool: &MySqlPool,
    user_id: i32,
    start: chrono::NaiveDateTime,
    end: chrono::NaiveDateTime,
) -> Result<Vec<chrono::NaiveDate>> {
    let rows = sqlx::query(
        "SELECT DISTINCT DATE(answered_at) AS d FROM question_records \
        WHERE user_id = ? AND answered_at >= ? AND answered_at < ?",
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
    .context("查询答题日期失败")?;
    let mut dates = Vec::new();
    for row in rows {
        let d: chrono::NaiveDate = row.try_get("d").context("解析答题日期失败")?;
        dates.push(d);
    }
    Ok(dates)
}

// ============ 工具模块：学习笔记 ============

#[derive(sqlx::FromRow)]
pub struct Note {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub content: Option<String>,
    pub tags: Option<String>,
    pub is_pinned: bool,
    pub source_type: String,
    pub source_id: Option<i32>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

pub async fn find_note_by_id(pool: &MySqlPool, id: i32) -> Result<Option<Note>> {
    let row = sqlx::query_as::<_, Note>(
        "SELECT id, user_id, title, CAST(content AS CHAR) AS content, CAST(tags AS CHAR) AS tags, is_pinned, source_type, source_id, created_at, updated_at \
        FROM notes WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("查询笔记失败")?;
    Ok(row)
}

pub async fn count_notes(
    pool: &MySqlPool,
    user_id: i32,
    tag: Option<&str>,
    keyword: Option<&str>,
) -> Result<i64> {
    let mut sql = String::from("SELECT COUNT(*) AS cnt FROM notes WHERE user_id = ?");
    if tag.is_some() {
        sql.push_str(" AND JSON_CONTAINS(tags, JSON_QUOTE(?))");
    }
    if keyword.is_some() {
        sql.push_str(
            " AND (title LIKE CONCAT('%', ?, '%') OR content LIKE CONCAT('%', ?, '%'))",
        );
    }
    let mut query = sqlx::query(&sql).bind(user_id);
    if let Some(t) = tag {
        query = query.bind(t);
    }
    if let Some(k) = keyword {
        query = query.bind(k).bind(k);
    }
    let row = query.fetch_one(pool).await.context("统计笔记数量失败")?;
    let total = row
        .try_get::<i64, _>("cnt")
        .ok()
        .or_else(|| row.try_get::<Option<i64>, _>("cnt").ok().flatten())
        .unwrap_or(0);
    Ok(total)
}

pub async fn find_notes(
    pool: &MySqlPool,
    user_id: i32,
    tag: Option<&str>,
    keyword: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Note>> {
    let mut sql = String::from(
        "SELECT id, user_id, title, CAST(content AS CHAR) AS content, CAST(tags AS CHAR) AS tags, is_pinned, source_type, source_id, created_at, updated_at \
        FROM notes WHERE user_id = ?",
    );
    if tag.is_some() {
        sql.push_str(" AND JSON_CONTAINS(tags, JSON_QUOTE(?))");
    }
    if keyword.is_some() {
        sql.push_str(
            " AND (title LIKE CONCAT('%', ?, '%') OR content LIKE CONCAT('%', ?, '%'))",
        );
    }
    sql.push_str(" ORDER BY is_pinned DESC, updated_at DESC, id DESC LIMIT ? OFFSET ?");

    let mut query = sqlx::query_as::<_, Note>(&sql).bind(user_id);
    if let Some(t) = tag {
        query = query.bind(t);
    }
    if let Some(k) = keyword {
        query = query.bind(k).bind(k);
    }
    let rows = query
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("查询笔记列表失败")?;
    Ok(rows)
}

pub async fn create_note(
    pool: &MySqlPool,
    user_id: i32,
    title: &str,
    content: Option<&str>,
    tags: Option<&str>,
    source_type: &str,
    source_id: Option<i32>,
) -> Result<Note> {
    let result = sqlx::query(
        "INSERT INTO notes (user_id, title, content, tags, source_type, source_id) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(title)
    .bind(content)
    .bind(tags)
    .bind(source_type)
    .bind(source_id)
    .execute(pool)
    .await
    .context("插入笔记失败")?;
    let id = result.last_insert_id() as i32;
    let note = find_note_by_id(pool, id)
        .await?
        .context("查询新建笔记失败")?;
    Ok(note)
}

pub async fn update_note(
    pool: &MySqlPool,
    id: i32,
    user_id: i32,
    title: &str,
    content: Option<&str>,
    tags: Option<&str>,
    is_pinned: bool,
) -> Result<Option<Note>> {
    let result = sqlx::query(
        "UPDATE notes SET title = ?, content = ?, tags = ?, is_pinned = ?, updated_at = NOW() \
        WHERE id = ? AND user_id = ?",
    )
    .bind(title)
    .bind(content)
    .bind(tags)
    .bind(is_pinned)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .context("更新笔记失败")?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    find_note_by_id(pool, id).await
}

pub async fn delete_note(pool: &MySqlPool, id: i32, user_id: i32) -> Result<bool> {
    let result = sqlx::query("DELETE FROM notes WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("删除笔记失败")?;
    Ok(result.rows_affected() > 0)
}

pub async fn count_notes_by_user(pool: &MySqlPool, user_id: i32) -> Result<i64> {
    count_notes(pool, user_id, None, None).await
}

pub async fn find_note_tags_by_user(pool: &MySqlPool, user_id: i32) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT CAST(tags AS CHAR) AS tags FROM notes WHERE user_id = ? AND tags IS NOT NULL",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("查询笔记标签失败")?;
    let mut tags = Vec::new();
    for row in rows {
        let t: String = row.try_get("tags").context("解析笔记标签失败")?;
        tags.push(t);
    }
    Ok(tags)
}

// ============ 工具模块：绩点计算器 ============

#[derive(sqlx::FromRow)]
pub struct Semester {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub year: i32,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(sqlx::FromRow)]
pub struct SemesterWithCount {
    pub id: i32,
    pub name: String,
    pub year: i32,
    pub course_count: i64,
}

#[derive(sqlx::FromRow)]
pub struct CourseGrade {
    pub id: i32,
    pub semester_id: i32,
    pub name: String,
    pub credit: String,
    pub score: Option<String>,
    pub grade: Option<String>,
    pub r#type: String,
    pub gpa: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

pub async fn find_semesters_with_counts(
    pool: &MySqlPool,
    user_id: i32,
) -> Result<Vec<SemesterWithCount>> {
    let rows = sqlx::query_as::<_, SemesterWithCount>(
        "SELECT s.id, s.name, s.year, COUNT(c.id) AS course_count \
        FROM semesters s LEFT JOIN course_grades c ON c.semester_id = s.id \
        WHERE s.user_id = ? \
        GROUP BY s.id, s.name, s.year, s.created_at \
        ORDER BY s.year DESC, s.created_at DESC, s.id DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("查询学期列表失败")?;
    Ok(rows)
}

pub async fn find_semester_by_id(pool: &MySqlPool, id: i32) -> Result<Option<Semester>> {
    let row = sqlx::query_as::<_, Semester>(
        "SELECT id, user_id, name, year, created_at FROM semesters WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("查询学期失败")?;
    Ok(row)
}

pub async fn create_semester(
    pool: &MySqlPool,
    user_id: i32,
    name: &str,
    year: i32,
) -> Result<Semester> {
    let result = sqlx::query("INSERT INTO semesters (user_id, name, year) VALUES (?, ?, ?)")
        .bind(user_id)
        .bind(name)
        .bind(year)
        .execute(pool)
        .await
        .context("创建学期失败")?;
    let id = result.last_insert_id() as i32;
    let semester = find_semester_by_id(pool, id)
        .await?
        .context("查询新建学期失败")?;
    Ok(semester)
}

pub async fn delete_semester_with_courses(
    pool: &MySqlPool,
    semester_id: i32,
) -> Result<bool> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM course_grades WHERE semester_id = ?")
        .bind(semester_id)
        .execute(&mut *tx)
        .await
        .context("删除学期课程失败")?;
    let result = sqlx::query("DELETE FROM semesters WHERE id = ?")
        .bind(semester_id)
        .execute(&mut *tx)
        .await
        .context("删除学期失败")?;
    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}

pub async fn find_courses_by_semester(
    pool: &MySqlPool,
    semester_id: i32,
) -> Result<Vec<CourseGrade>> {
    let rows = sqlx::query_as::<_, CourseGrade>(
        "SELECT id, semester_id, name, CAST(credit AS CHAR) AS credit, CAST(score AS CHAR) AS score, grade, type, CAST(gpa AS CHAR) AS gpa, created_at, updated_at \
        FROM course_grades WHERE semester_id = ? ORDER BY id ASC",
    )
    .bind(semester_id)
    .fetch_all(pool)
    .await
    .context("查询课程列表失败")?;
    Ok(rows)
}

pub async fn find_course_by_id(pool: &MySqlPool, id: i32) -> Result<Option<CourseGrade>> {
    let row = sqlx::query_as::<_, CourseGrade>(
        "SELECT id, semester_id, name, CAST(credit AS CHAR) AS credit, CAST(score AS CHAR) AS score, grade, type, CAST(gpa AS CHAR) AS gpa, created_at, updated_at \
        FROM course_grades WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("查询课程失败")?;
    Ok(row)
}

pub async fn create_course(
    pool: &MySqlPool,
    semester_id: i32,
    name: &str,
    credit: &str,
    score: Option<&str>,
    grade: Option<&str>,
    r#type: &str,
    gpa: Option<&str>,
) -> Result<CourseGrade> {
    let result = sqlx::query(
        "INSERT INTO course_grades (semester_id, name, credit, score, grade, type, gpa) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(semester_id)
    .bind(name)
    .bind(credit)
    .bind(score)
    .bind(grade)
    .bind(r#type)
    .bind(gpa)
    .execute(pool)
    .await
    .context("创建课程失败")?;
    let id = result.last_insert_id() as i32;
    let course = find_course_by_id(pool, id)
        .await?
        .context("查询新建课程失败")?;
    Ok(course)
}

pub async fn update_course(
    pool: &MySqlPool,
    id: i32,
    semester_id: i32,
    name: &str,
    credit: &str,
    score: Option<&str>,
    grade: Option<&str>,
    r#type: &str,
    gpa: Option<&str>,
) -> Result<Option<CourseGrade>> {
    let result = sqlx::query(
        "UPDATE course_grades SET semester_id = ?, name = ?, credit = ?, score = ?, grade = ?, type = ?, gpa = ?, updated_at = NOW() WHERE id = ?",
    )
    .bind(semester_id)
    .bind(name)
    .bind(credit)
    .bind(score)
    .bind(grade)
    .bind(r#type)
    .bind(gpa)
    .bind(id)
    .execute(pool)
    .await
    .context("更新课程失败")?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    find_course_by_id(pool, id).await
}

pub async fn delete_course(pool: &MySqlPool, id: i32) -> Result<bool> {
    let result = sqlx::query("DELETE FROM course_grades WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("删除课程失败")?;
    Ok(result.rows_affected() > 0)
}

pub async fn count_courses_by_semester(pool: &MySqlPool, semester_id: i32) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM course_grades WHERE semester_id = ?")
        .bind(semester_id)
        .fetch_one(pool)
        .await
        .context("统计课程数量失败")?;
    let total = row
        .try_get::<i64, _>("cnt")
        .ok()
        .or_else(|| row.try_get::<Option<i64>, _>("cnt").ok().flatten())
        .unwrap_or(0);
    Ok(total)
}

// ============ 工具模块：个人文档库 ============

#[derive(sqlx::FromRow)]
pub struct Document {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub file_url: String,
    pub file_size: i64,
    pub file_type: String,
    pub category: Option<String>,
    pub is_offline: bool,
    pub last_opened_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(sqlx::FromRow)]
pub struct DocumentCategoryCount {
    pub category: Option<String>,
    pub count: i64,
}

pub async fn find_document_by_id(pool: &MySqlPool, id: i32) -> Result<Option<Document>> {
    let row = sqlx::query_as::<_, Document>(
        "SELECT id, user_id, name, file_url, file_size, file_type, category, is_offline, last_opened_at, created_at, updated_at \
        FROM documents WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("查询文档失败")?;
    Ok(row)
}

pub async fn count_documents(
    pool: &MySqlPool,
    user_id: i32,
    category: Option<&str>,
) -> Result<i64> {
    let mut sql = String::from("SELECT COUNT(*) AS cnt FROM documents WHERE user_id = ?");
    if let Some(c) = category {
        if c != "全部" {
            sql.push_str(" AND category = ?");
        }
    }
    let mut query = sqlx::query(&sql).bind(user_id);
    if let Some(c) = category {
        if c != "全部" {
            query = query.bind(c);
        }
    }
    let row = query.fetch_one(pool).await.context("统计文档数量失败")?;
    let total = row
        .try_get::<i64, _>("cnt")
        .ok()
        .or_else(|| row.try_get::<Option<i64>, _>("cnt").ok().flatten())
        .unwrap_or(0);
    Ok(total)
}

pub async fn find_documents(
    pool: &MySqlPool,
    user_id: i32,
    category: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Document>> {
    let mut sql = String::from(
        "SELECT id, user_id, name, file_url, file_size, file_type, category, is_offline, last_opened_at, created_at, updated_at \
        FROM documents WHERE user_id = ?",
    );
    let mut has_category = false;
    if let Some(c) = category {
        if c != "全部" {
            sql.push_str(" AND category = ?");
            has_category = true;
        }
    }
    sql.push_str(" ORDER BY updated_at DESC, id DESC LIMIT ? OFFSET ?");
    let mut query = sqlx::query_as::<_, Document>(&sql).bind(user_id);
    if has_category {
        query = query.bind(category.unwrap_or_default());
    }
    let rows = query
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("查询文档列表失败")?;
    Ok(rows)
}

pub async fn sum_document_sizes(pool: &MySqlPool, user_id: i32) -> Result<i64> {
    let row = sqlx::query(
        "SELECT COALESCE(SUM(file_size), 0) AS total FROM documents WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .context("统计文档存储大小失败")?;
    let total = row
        .try_get::<i64, _>("total")
        .ok()
        .or_else(|| row.try_get::<Option<i64>, _>("total").ok().flatten())
        .unwrap_or(0);
    Ok(total)
}

pub async fn find_document_category_counts(
    pool: &MySqlPool,
    user_id: i32,
) -> Result<Vec<DocumentCategoryCount>> {
    let rows = sqlx::query_as::<_, DocumentCategoryCount>(
        "SELECT category, COUNT(*) AS count FROM documents WHERE user_id = ? GROUP BY category",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("统计文档分类失败")?;
    Ok(rows)
}

pub async fn update_document_offline(
    pool: &MySqlPool,
    id: i32,
    user_id: i32,
    is_offline: bool,
) -> Result<Option<Document>> {
    let result = sqlx::query(
        "UPDATE documents SET is_offline = ?, updated_at = NOW() WHERE id = ? AND user_id = ?",
    )
    .bind(is_offline)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .context("更新文档离线状态失败")?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    find_document_by_id(pool, id).await
}

pub async fn delete_document(pool: &MySqlPool, id: i32, user_id: i32) -> Result<bool> {
    let result = sqlx::query("DELETE FROM documents WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("删除文档失败")?;
    Ok(result.rows_affected() > 0)
}

// ============ 工具模块：精选书签 ============

#[derive(sqlx::FromRow)]
pub struct Bookmark {
    pub id: i32,
    pub user_id: i32,
    pub quote: String,
    pub source_title: String,
    pub source_url: Option<String>,
    pub source_type: String,
    pub source_id: Option<i32>,
    pub anchor: Option<String>,
    pub note: Option<String>,
    pub color: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

pub async fn find_bookmark_by_id(pool: &MySqlPool, id: i32) -> Result<Option<Bookmark>> {
    let row = sqlx::query_as::<_, Bookmark>(
        "SELECT id, user_id, quote, source_title, source_url, source_type, source_id, anchor, note, color, created_at, updated_at \
        FROM bookmarks WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("查询书签失败")?;
    Ok(row)
}

pub async fn count_bookmarks(
    pool: &MySqlPool,
    user_id: i32,
    color: Option<&str>,
    keyword: Option<&str>,
) -> Result<i64> {
    let mut sql = String::from("SELECT COUNT(*) AS cnt FROM bookmarks WHERE user_id = ?");
    if color.is_some() {
        sql.push_str(" AND color = ?");
    }
    if keyword.is_some() {
        sql.push_str(
            " AND (quote LIKE CONCAT('%', ?, '%') OR note LIKE CONCAT('%', ?, '%'))",
        );
    }
    let mut query = sqlx::query(&sql).bind(user_id);
    if let Some(c) = color {
        query = query.bind(c);
    }
    if let Some(k) = keyword {
        query = query.bind(k).bind(k);
    }
    let row = query.fetch_one(pool).await.context("统计书签数量失败")?;
    let total = row
        .try_get::<i64, _>("cnt")
        .ok()
        .or_else(|| row.try_get::<Option<i64>, _>("cnt").ok().flatten())
        .unwrap_or(0);
    Ok(total)
}

pub async fn find_bookmarks(
    pool: &MySqlPool,
    user_id: i32,
    color: Option<&str>,
    keyword: Option<&str>,
) -> Result<Vec<Bookmark>> {
    let mut sql = String::from(
        "SELECT id, user_id, quote, source_title, source_url, source_type, source_id, anchor, note, color, created_at, updated_at \
        FROM bookmarks WHERE user_id = ?",
    );
    if color.is_some() {
        sql.push_str(" AND color = ?");
    }
    if keyword.is_some() {
        sql.push_str(
            " AND (quote LIKE CONCAT('%', ?, '%') OR note LIKE CONCAT('%', ?, '%'))",
        );
    }
    sql.push_str(" ORDER BY created_at DESC, id DESC");
    let mut query = sqlx::query_as::<_, Bookmark>(&sql).bind(user_id);
    if let Some(c) = color {
        query = query.bind(c);
    }
    if let Some(k) = keyword {
        query = query.bind(k).bind(k);
    }
    let rows = query.fetch_all(pool).await.context("查询书签列表失败")?;
    Ok(rows)
}

pub async fn create_bookmark(
    pool: &MySqlPool,
    user_id: i32,
    quote: &str,
    source_title: &str,
    source_url: Option<&str>,
    source_type: &str,
    source_id: Option<i32>,
    anchor: Option<&str>,
    note: Option<&str>,
    color: &str,
) -> Result<Bookmark> {
    let result = sqlx::query(
        "INSERT INTO bookmarks (user_id, quote, source_title, source_url, source_type, source_id, anchor, note, color) \
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(quote)
    .bind(source_title)
    .bind(source_url)
    .bind(source_type)
    .bind(source_id)
    .bind(anchor)
    .bind(note)
    .bind(color)
    .execute(pool)
    .await
    .context("创建书签失败")?;
    let id = result.last_insert_id() as i32;
    let bookmark = find_bookmark_by_id(pool, id)
        .await?
        .context("查询新建书签失败")?;
    Ok(bookmark)
}

pub async fn update_bookmark(
    pool: &MySqlPool,
    id: i32,
    user_id: i32,
    quote: &str,
    source_title: &str,
    source_url: Option<&str>,
    source_type: &str,
    source_id: Option<i32>,
    anchor: Option<&str>,
    note: Option<&str>,
    color: &str,
) -> Result<Option<Bookmark>> {
    let result = sqlx::query(
        "UPDATE bookmarks SET quote = ?, source_title = ?, source_url = ?, source_type = ?, source_id = ?, anchor = ?, note = ?, color = ? \
        WHERE id = ? AND user_id = ?",
    )
    .bind(quote)
    .bind(source_title)
    .bind(source_url)
    .bind(source_type)
    .bind(source_id)
    .bind(anchor)
    .bind(note)
    .bind(color)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .context("更新书签失败")?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    find_bookmark_by_id(pool, id).await
}

pub async fn delete_bookmark(pool: &MySqlPool, id: i32, user_id: i32) -> Result<bool> {
    let result = sqlx::query("DELETE FROM bookmarks WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("删除书签失败")?;
    Ok(result.rows_affected() > 0)
}

pub async fn count_bookmarks_by_user(pool: &MySqlPool, user_id: i32) -> Result<i64> {
    count_bookmarks(pool, user_id, None, None).await
}

// ============ 工具模块：资料导出 ============

#[derive(sqlx::FromRow)]
pub struct ExportRecord {
    pub id: i32,
    pub user_id: i32,
    pub file_ids: Option<String>,
    pub format: String,
    pub template: String,
    pub file_url: String,
    pub file_size: i64,
    pub created_at: chrono::NaiveDateTime,
}

pub async fn create_export_record(
    pool: &MySqlPool,
    user_id: i32,
    file_ids: &str,
    format: &str,
    template: &str,
    file_url: &str,
    file_size: i64,
) -> Result<ExportRecord> {
    let result = sqlx::query(
        "INSERT INTO export_records (user_id, file_ids, format, template, file_url, file_size) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(file_ids)
    .bind(format)
    .bind(template)
    .bind(file_url)
    .bind(file_size)
    .execute(pool)
    .await
    .context("创建导出记录失败")?;
    let id = result.last_insert_id() as i32;
    let row = sqlx::query_as::<_, ExportRecord>(
        "SELECT id, user_id, CAST(file_ids AS CHAR) AS file_ids, format, template, file_url, file_size, created_at \
        FROM export_records WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .context("查询导出记录失败")?;
    Ok(row)
}

pub async fn find_export_records(
    pool: &MySqlPool,
    user_id: i32,
    limit: i64,
    offset: i64,
) -> Result<Vec<ExportRecord>> {
    let rows = sqlx::query_as::<_, ExportRecord>(
        "SELECT id, user_id, CAST(file_ids AS CHAR) AS file_ids, format, template, file_url, file_size, created_at \
        FROM export_records WHERE user_id = ? ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("查询导出记录失败")?;
    Ok(rows)
}

pub async fn count_export_records(pool: &MySqlPool, user_id: i32) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM export_records WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .context("统计导出记录失败")?;
    let total = row
        .try_get::<i64, _>("cnt")
        .ok()
        .or_else(|| row.try_get::<Option<i64>, _>("cnt").ok().flatten())
        .unwrap_or(0);
    Ok(total)
}
