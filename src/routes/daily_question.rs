use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    state::AppState,
};

fn parse_options(raw: &Option<String>) -> Vec<String> {
    raw.as_deref()
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
}

fn days_in_month(year: i32, month: u32) -> Option<u32> {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)?
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)?
    };
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    Some((next - first).num_days() as u32)
}

fn compute_answer_streak(dates: &[NaiveDate], today: NaiveDate) -> i32 {
    let yesterday = today - chrono::Duration::days(1);
    let anchor = if dates.contains(&today) {
        today
    } else if dates.contains(&yesterday) {
        yesterday
    } else {
        return 0;
    };
    let mut streak = 0;
    loop {
        let day = anchor - chrono::Duration::days(streak as i64);
        if dates.contains(&day) {
            streak += 1;
        } else {
            break;
        }
    }
    streak
}

async fn current_answer_streak(pool: &sqlx::MySqlPool, user_id: i32) -> anyhow::Result<i32> {
    let today = chrono::Local::now().naive_local().date();
    let since = today - chrono::Duration::days(366);
    let dates = db::find_question_answer_dates(pool, user_id, since).await?;
    Ok(compute_answer_streak(&dates, today))
}

// ============ 今日题目 ============

#[derive(Serialize)]
pub struct TodayRecord {
    pub selected: String,
    pub is_correct: bool,
    pub explanation: Option<String>,
}

#[derive(Serialize)]
pub struct TodayData {
    pub id: i32,
    pub subject: String,
    pub question: String,
    pub options: Vec<String>,
    pub difficulty: i32,
    pub date: String,
    pub has_answered: bool,
    pub streak: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<TodayRecord>,
}

/// GET /api/daily-question/today
pub async fn get_today(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let today = chrono::Local::now().naive_local().date();
    let question = match db::find_daily_question_by_date(pool, today).await {
        Ok(Some(q)) => q,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "今日暂无题目");
        }
        Err(e) => {
            tracing::error!("查询今日每日一题失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let record = match db::find_question_record(pool, user_id, question.id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("查询答题记录失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let streak = match current_answer_streak(pool, user_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("计算连续答题天数失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let record_data = record.as_ref().map(|r| TodayRecord {
        selected: r.selected.clone(),
        is_correct: r.is_correct,
        explanation: question.explanation.clone(),
    });
    response::ok(
        StatusCode::OK,
        200,
        "success",
        TodayData {
            id: question.id,
            subject: question.subject,
            question: question.question,
            options: parse_options(&question.options),
            difficulty: question.difficulty,
            date: question.date.format("%Y-%m-%d").to_string(),
            has_answered: record.is_some(),
            streak,
            record: record_data,
        },
    )
}

// ============ 提交答案 ============

#[derive(Deserialize)]
pub struct AnswerRequest {
    pub question_id: i32,
    pub selected: String,
}

#[derive(Serialize)]
pub struct AnswerData {
    pub is_correct: bool,
    pub streak: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correct_answer: Option<String>,
    pub explanation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub earned_points: Option<i32>,
}

/// POST /api/daily-question/answer
pub async fn submit_answer(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<AnswerRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let selected = payload.selected.trim().to_uppercase();
    if selected.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "请选择答案");
    }
    let question = match db::find_daily_question_by_id(pool, payload.question_id).await {
        Ok(Some(q)) => q,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "题目不存在");
        }
        Err(e) => {
            tracing::error!("查询题目失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    if let Ok(Some(_)) = db::find_question_record(pool, user_id, question.id).await {
        return response::error(StatusCode::CONFLICT, 409, "今日已答题，不可重复提交");
    }

    let is_correct = selected == question.answer.trim().to_uppercase();
    let record = match db::create_question_record(pool, user_id, question.id, &selected, is_correct)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            if db::is_duplicate_key_anyhow(&e) {
                return response::error(
                    StatusCode::CONFLICT,
                    409,
                    "今日已答题，不可重复提交",
                );
            }
            tracing::error!("保存答题记录失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let streak = match current_answer_streak(pool, user_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("计算连续答题天数失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    if record.is_correct {
        const EARNED_POINTS: i32 = 10;
        if let Err(e) = db::add_user_points(pool, user_id, EARNED_POINTS).await {
            tracing::warn!("答题积分奖励失败: {}", e);
        }
        response::ok(
            StatusCode::OK,
            200,
            format!("答对啦！+{} 积分", EARNED_POINTS),
            AnswerData {
                is_correct: true,
                streak,
                correct_answer: None,
                explanation: question.explanation.clone(),
                earned_points: Some(EARNED_POINTS),
            },
        )
    } else {
        response::ok(
            StatusCode::OK,
            200,
            format!("答错啦！正确答案是 {}", question.answer),
            AnswerData {
                is_correct: false,
                streak,
                correct_answer: Some(question.answer.clone()),
                explanation: question.explanation.clone(),
                earned_points: None,
            },
        )
    }
}

// ============ 答题历史 ============

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub month: Option<String>,
    pub only_wrong: Option<bool>,
}

#[derive(Serialize)]
pub struct HistoryItem {
    pub id: i32,
    pub question_id: i32,
    pub subject: String,
    pub selected: String,
    pub is_correct: bool,
    pub answered_at: String,
}

#[derive(Serialize)]
pub struct CalendarDay {
    pub date: String,
    pub has_answered: bool,
}

#[derive(Serialize)]
pub struct HistoryData {
    pub list: Vec<HistoryItem>,
    pub calendar: Vec<CalendarDay>,
}

/// GET /api/daily-question/history
pub async fn get_history(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(query): Query<HistoryQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let now = chrono::Local::now().naive_local();
    let month = query.month.unwrap_or_else(|| now.format("%Y-%m").to_string());
    let parts: Vec<&str> = month.split('-').collect();
    if parts.len() != 2 {
        return response::error(
            StatusCode::BAD_REQUEST,
            400,
            "月份格式无效，应为YYYY-MM",
        );
    }
    let (Ok(year), Ok(month_num)) = (
        parts[0].parse::<i32>(),
        parts[1].parse::<u32>(),
    ) else {
        return response::error(
            StatusCode::BAD_REQUEST,
            400,
            "月份格式无效，应为YYYY-MM",
        );
    };
    let Some(start_date) = NaiveDate::from_ymd_opt(year, month_num, 1) else {
        return response::error(StatusCode::BAD_REQUEST, 400, "月份格式无效");
    };
    let end_date = if month_num == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month_num + 1, 1)
    }
    .unwrap_or(start_date + chrono::Duration::days(31));
    let start_dt: NaiveDateTime = start_date.and_hms_opt(0, 0, 0).unwrap();
    let end_dt: NaiveDateTime = end_date.and_hms_opt(0, 0, 0).unwrap();

    let only_wrong = query.only_wrong.unwrap_or(false);
    let records = match db::find_question_records_by_month(
        pool,
        user_id,
        start_dt,
        end_dt,
        only_wrong,
    )
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("查询答题历史失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let answered_dates = match db::find_question_dates_by_month(pool, user_id, start_dt, end_dt).await
    {
        Ok(dates) => dates,
        Err(e) => {
            tracing::error!("查询答题日期失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let list: Vec<HistoryItem> = records
        .into_iter()
        .map(|r| HistoryItem {
            id: r.id,
            question_id: r.question_id,
            subject: r.subject,
            selected: r.selected,
            is_correct: r.is_correct,
            answered_at: r.answered_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        })
        .collect();

    let mut calendar = Vec::new();
    if let Some(dim) = days_in_month(year, month_num) {
        for day in 1..=dim {
            if let Some(date) = NaiveDate::from_ymd_opt(year, month_num, day) {
                calendar.push(CalendarDay {
                    date: date.format("%Y-%m-%d").to_string(),
                    has_answered: answered_dates.contains(&date),
                });
            }
        }
    }

    response::ok(
        StatusCode::OK,
        200,
        "success",
        HistoryData { list, calendar },
    )
}
