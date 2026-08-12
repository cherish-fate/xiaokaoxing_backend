use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use chrono::Datelike;
use serde::{Deserialize, Serialize};

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    state::AppState,
};

// ============ 共享工具 ============

/// 解析 JSON 数组字段（subjects/tags），失败或为空返回空数组
pub fn parse_json_array(s: &Option<String>) -> Vec<String> {
    s.as_deref()
        .and_then(|json| serde_json::from_str::<Vec<String>>(json).ok())
        .unwrap_or_default()
}

/// 当前用户的打卡统计（累计天数、连续天数、本校排名）
pub struct UserCheckinStats {
    pub total_days: i64,
    pub continuous_days: i32,
    pub rank: i64,
}

/// 计算用户打卡统计：累计天数、当前连续天数（昨日或今日有打卡才算存活）、本校排名
pub async fn compute_user_checkin_stats(
    pool: &sqlx::MySqlPool,
    user_id: i32,
    school_name: &str,
) -> anyhow::Result<UserCheckinStats> {
    let total_days = db::count_checkins_by_user(pool, user_id).await?;

    let today = chrono::Local::now().naive_local().date();
    let yesterday = today - chrono::Duration::days(1);
    let continuous_days = match db::find_latest_checkin_by_user(pool, user_id).await? {
        Some(rec) if rec.checkin_date >= yesterday => rec.continuous_days,
        _ => 0,
    };

    // 本校排名：统计严格领先于当前用户的数量 + 1
    let stats = db::find_school_checkin_stats(pool, school_name).await?;
    let ahead = stats
        .iter()
        .filter(|s| {
            s.user_id != user_id
                && (s.continuous_days > continuous_days
                    || (s.continuous_days == continuous_days && s.total_days > total_days))
        })
        .count() as i64;
    let rank = ahead + 1;

    Ok(UserCheckinStats {
        total_days,
        continuous_days,
        rank,
    })
}

// ============ 今日打卡状态 ============

#[derive(Serialize)]
pub struct CheckinRecordData {
    pub id: i32,
    pub subjects: Vec<String>,
    pub duration: Option<String>,
    pub note: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Serialize)]
pub struct TodayData {
    pub has_checked_in: bool,
    pub continuous_days: i32,
    pub total_days: i64,
    pub rank: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkin_record: Option<CheckinRecordData>,
}

/// GET /api/checkin/today — 获取今日打卡状态
pub async fn get_today(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let user = match db::find_user_by_id(pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "用户不存在"),
        Err(e) => {
            tracing::error!("查询用户失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let today = chrono::Local::now().naive_local().date();
    let today_record = match db::find_checkin_by_user_date(pool, user_id, today).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("查询今日打卡失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let stats = match compute_user_checkin_stats(pool, user_id, &user.school_name).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("计算打卡统计失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let checkin_record = today_record.as_ref().map(|r| CheckinRecordData {
        id: r.id,
        subjects: parse_json_array(&r.subjects),
        duration: r.duration.clone(),
        note: r.note.clone(),
        tags: parse_json_array(&r.tags),
    });

    response::ok(
        StatusCode::OK,
        200,
        "success",
        TodayData {
            has_checked_in: today_record.is_some(),
            continuous_days: stats.continuous_days,
            total_days: stats.total_days,
            rank: stats.rank,
            checkin_record,
        },
    )
}

// ============ 打卡 ============

#[derive(Deserialize)]
pub struct CreateCheckinRequest {
    pub subjects: Option<Vec<String>>,
    pub duration: Option<String>,
    pub note: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct CreateCheckinData {
    pub id: i32,
    pub checkin_date: String,
    pub continuous_days: i32,
    pub total_days: i64,
    pub earned_points: i32,
}

/// POST /api/checkin — 提交打卡记录
pub async fn create_checkin(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreateCheckinRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    // 备注长度校验
    if let Some(note) = payload.note.as_deref() {
        if note.chars().count() > 100 {
            return response::error(StatusCode::BAD_REQUEST, 400, "备注最多100字");
        }
    }

    let user = match db::find_user_by_id(pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "用户不存在"),
        Err(e) => {
            tracing::error!("查询用户失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let today = chrono::Local::now().naive_local().date();

    // 预检：今日是否已打卡
    match db::find_checkin_by_user_date(pool, user_id, today).await {
        Ok(Some(_)) => {
            return response::error(
                StatusCode::CONFLICT,
                409,
                "今日已打卡，明天继续加油！",
            );
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("查询今日打卡失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    }

    let subjects_json = payload
        .subjects
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| serde_json::to_string(v).unwrap_or_default());
    let tags_json = payload
        .tags
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| serde_json::to_string(v).unwrap_or_default());
    let duration = payload
        .duration
        .as_deref()
        .map(|d| d.trim())
        .filter(|d| !d.is_empty());
    let note = payload
        .note
        .as_deref()
        .map(|n| n.trim())
        .filter(|n| !n.is_empty());

    let record = match db::create_checkin(
        pool,
        user_id,
        today,
        subjects_json.as_deref(),
        duration,
        note,
        tags_json.as_deref(),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            if db::is_duplicate_key_anyhow(&e) {
                return response::error(
                    StatusCode::CONFLICT,
                    409,
                    "今日已打卡，明天继续加油！",
                );
            }
            tracing::error!("创建打卡记录失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 异步积分奖励（不阻塞主流程，失败仅记录日志）
    const EARNED_POINTS: i32 = 5;
    if let Err(e) = db::add_user_points(pool, user_id, EARNED_POINTS).await {
        tracing::warn!("打卡积分奖励失败: {}", e);
    }

    let total_days = match db::count_checkins_by_user(pool, user_id).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!("统计打卡天数失败: {}", e);
            0
        }
    };

    let message = format!("打卡成功，已连续打卡{}天！", record.continuous_days);

    response::ok(
        StatusCode::CREATED,
        201,
        message,
        CreateCheckinData {
            id: record.id,
            checkin_date: record.checkin_date.format("%Y-%m-%d").to_string(),
            continuous_days: record.continuous_days,
            total_days,
            earned_points: EARNED_POINTS,
        },
    )
}

// ============ 打卡日历 ============

#[derive(Deserialize)]
pub struct CalendarQuery {
    pub year: Option<i32>,
    pub month: Option<u32>,
}

#[derive(Serialize)]
pub struct CalendarDay {
    pub date: String,
    pub day: u32,
    pub is_checked: bool,
    pub is_today: bool,
}

#[derive(Serialize)]
pub struct CalendarData {
    pub year: i32,
    pub month: u32,
    pub days: Vec<CalendarDay>,
}

/// 计算某年某月的天数
fn days_in_month(year: i32, month: u32) -> Option<u32> {
    let next = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)?
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)?
    };
    let first = chrono::NaiveDate::from_ymd_opt(year, month, 1)?;
    Some((next - first).num_days() as u32)
}

/// GET /api/checkin/calendar — 获取指定月份打卡日历
pub async fn get_calendar(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(params): Query<CalendarQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let now = chrono::Local::now().naive_local().date();
    let year = params.year.unwrap_or(now.year());
    let month = params.month.unwrap_or(now.month());

    let dim = match days_in_month(year, month) {
        Some(d) => d,
        None => return response::error(StatusCode::BAD_REQUEST, 400, "年月参数无效"),
    };

    let checked_dates = match db::find_checkin_dates_by_user_month(pool, user_id, year, month).await
    {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("查询月度打卡日期失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let today = chrono::Local::now().naive_local().date();
    let mut days = Vec::with_capacity(dim as usize);
    for day in 1..=dim {
        let date = match chrono::NaiveDate::from_ymd_opt(year, month, day) {
            Some(d) => d,
            None => continue,
        };
        days.push(CalendarDay {
            date: date.format("%Y-%m-%d").to_string(),
            day,
            is_checked: checked_dates.contains(&date),
            is_today: date == today,
        });
    }

    response::ok(
        StatusCode::OK,
        200,
        "success",
        CalendarData {
            year,
            month,
            days,
        },
    )
}

// ============ 打卡排行 ============

#[derive(Deserialize)]
pub struct RankingQuery {
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct RankingItem {
    pub rank: i64,
    pub user_id: i32,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub continuous_days: i32,
    pub total_days: i64,
}

#[derive(Serialize)]
pub struct RankingData {
    pub list: Vec<RankingItem>,
    pub my_rank: RankingItem,
}

/// GET /api/checkin/ranking — 获取本校打卡排行 TOP50
pub async fn get_ranking(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(params): Query<RankingQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let user = match db::find_user_by_id(pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "用户不存在"),
        Err(e) => {
            tracing::error!("查询用户失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let limit = params.limit.unwrap_or(50).clamp(1, 50);

    let stats = match db::find_school_checkin_stats(pool, &user.school_name).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("查询同校打卡统计失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 构造排行列表（带名次），名次 = 1 + 严格领先人数
    let mut ranked: Vec<(i64, &db::CheckinStatRow)> = Vec::with_capacity(stats.len());
    for s in &stats {
        let ahead = stats
            .iter()
            .filter(|o| {
                o.user_id != s.user_id
                    && (o.continuous_days > s.continuous_days
                        || (o.continuous_days == s.continuous_days && o.total_days > s.total_days))
            })
            .count() as i64;
        ranked.push((ahead + 1, s));
    }

    let list: Vec<RankingItem> = ranked
        .iter()
        .take(limit as usize)
        .map(|(rank, s)| RankingItem {
            rank: *rank,
            user_id: s.user_id,
            nickname: s.nickname.clone(),
            avatar_url: s.avatar_url.clone(),
            continuous_days: s.continuous_days,
            total_days: s.total_days,
        })
        .collect();

    let my_rank = ranked
        .iter()
        .find(|(_, s)| s.user_id == user_id)
        .map(|(rank, s)| RankingItem {
            rank: *rank,
            user_id: s.user_id,
            nickname: s.nickname.clone(),
            avatar_url: s.avatar_url.clone(),
            continuous_days: s.continuous_days,
            total_days: s.total_days,
        })
        .unwrap_or(RankingItem {
            rank: 0,
            user_id,
            nickname: user.nickname.clone(),
            avatar_url: user.avatar_url.clone(),
            continuous_days: 0,
            total_days: 0,
        });

    response::ok(
        StatusCode::OK,
        200,
        "success",
        RankingData { list, my_rank },
    )
}
