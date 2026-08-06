use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    state::AppState,
};

#[derive(Deserialize)]
pub struct ExamListQuery {
    pub limit: Option<i64>,
    pub status: Option<String>,
}

#[derive(Serialize)]
pub struct ExamItem {
    pub id: i32,
    pub name: String,
    pub exam_date: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub location: Option<String>,
    pub is_completed: bool,
    pub days_remaining: i64,
    pub days_text: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct ExamListData {
    pub total: i64,
    pub list: Vec<ExamItem>,
}

#[derive(Deserialize)]
pub struct CreateExamRequest {
    pub name: String,
    pub exam_date: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub location: Option<String>,
    pub is_completed: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateExamRequest {
    pub name: Option<String>,
    pub exam_date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub location: Option<String>,
    pub is_completed: Option<bool>,
}

#[derive(Serialize)]
pub struct ExamData {
    pub id: i32,
    pub name: String,
    pub exam_date: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub location: Option<String>,
    pub is_completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

fn calc_days_text(exam_date: NaiveDate) -> (i64, String) {
    let today = chrono::Local::now().naive_local().date();
    let days = (exam_date - today).num_days();
    let text = if days == 0 {
        "今天".to_string()
    } else if days > 0 {
        format!("{}天后", days)
    } else {
        format!("已过期{}天", -days)
    };
    (days, text)
}

fn calc_exam_status(exam_date: NaiveDate, is_completed: bool) -> String {
    if is_completed {
        "completed".to_string()
    } else {
        let today = chrono::Local::now().naive_local().date();
        if exam_date > today {
            "upcoming".to_string()
        } else if exam_date == today {
            "today".to_string()
        } else {
            "completed".to_string()
        }
    }
}

/// GET /api/exams — 获取考试列表
pub async fn list_exams(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(params): Query<ExamListQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let exams = match db::find_exams_by_user(pool, user_id, params.status.as_deref()).await {
        Ok(exams) => exams,
        Err(e) => {
            tracing::error!("查询考试列表失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let total = exams.len() as i64;
    let limit = params.limit.unwrap_or(3);

    let list: Vec<ExamItem> = exams
        .into_iter()
        .take(if limit == 0 {
            total as usize
        } else {
            limit as usize
        })
        .map(|exam| {
            let (days_remaining, days_text) = calc_days_text(exam.exam_date);
            let status = calc_exam_status(exam.exam_date, exam.is_completed);
            ExamItem {
                id: exam.id,
                name: exam.name,
                exam_date: exam.exam_date.format("%Y-%m-%d").to_string(),
                start_time: exam.start_time.format("%H:%M").to_string(),
                end_time: exam.end_time.map(|t| t.format("%H:%M").to_string()),
                location: exam.location,
                is_completed: exam.is_completed,
                days_remaining,
                days_text,
                status,
            }
        })
        .collect();

    response::ok(
        StatusCode::OK,
        200,
        "success",
        ExamListData { total, list },
    )
}

/// POST /api/exams — 添加考试
pub async fn create_exam(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreateExamRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let name = payload.name.trim();
    if name.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "考试名称不能为空");
    }

    let exam_date: NaiveDate = match NaiveDate::parse_from_str(&payload.exam_date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            return response::error(
                StatusCode::BAD_REQUEST,
                400,
                "考试日期格式无效，应为 YYYY-MM-DD",
            );
        }
    };

    let start_time: NaiveTime = match NaiveTime::parse_from_str(&payload.start_time, "%H:%M") {
        Ok(t) => t,
        Err(_) => {
            return response::error(
                StatusCode::BAD_REQUEST,
                400,
                "开始时间格式无效，应为 HH:mm",
            );
        }
    };

    let end_time: Option<NaiveTime> = match payload.end_time {
        Some(ref t) if !t.is_empty() => match NaiveTime::parse_from_str(t, "%H:%M") {
            Ok(nt) => Some(nt),
            Err(_) => {
                return response::error(
                    StatusCode::BAD_REQUEST,
                    400,
                    "结束时间格式无效，应为 HH:mm",
                );
            }
        },
        _ => None,
    };

    let location = payload.location.as_deref().map(|l| l.trim()).filter(|l| !l.is_empty());
    let is_completed = payload.is_completed.unwrap_or(false);

    match db::create_exam(
        pool,
        user_id,
        name,
        exam_date,
        start_time,
        end_time,
        location,
        is_completed,
    )
    .await
    {
        Ok(exam) => {
            let created_at = exam.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            response::ok(
                StatusCode::CREATED,
                201,
                "考试添加成功",
                ExamData {
                    id: exam.id,
                    name: exam.name,
                    exam_date: exam.exam_date.format("%Y-%m-%d").to_string(),
                    start_time: exam.start_time.format("%H:%M").to_string(),
                    end_time: exam.end_time.map(|t| t.format("%H:%M").to_string()),
                    location: exam.location,
                    is_completed: exam.is_completed,
                    created_at: Some(created_at),
                    updated_at: None,
                },
            )
        }
        Err(e) => {
            tracing::error!("创建考试失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

/// PUT /api/exams/{id} — 更新考试
pub async fn update_exam(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateExamRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    // 验证考试存在且属于当前用户
    let exam = match db::find_exam_by_id(pool, id).await {
        Ok(Some(exam)) => exam,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "考试不存在或无权操作");
        }
        Err(e) => {
            tracing::error!("查询考试失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    if exam.user_id != user_id {
        return response::error(StatusCode::NOT_FOUND, 404, "考试不存在或无权操作");
    }

    // 解析可选字段
    let name = payload.name.as_deref().map(|n| n.trim()).filter(|n| !n.is_empty());

    let exam_date = match payload.exam_date {
        Some(ref d) if !d.is_empty() => match NaiveDate::parse_from_str(d, "%Y-%m-%d") {
            Ok(nd) => Some(nd),
            Err(_) => {
                return response::error(
                    StatusCode::BAD_REQUEST,
                    400,
                    "考试日期格式无效，应为 YYYY-MM-DD",
                );
            }
        },
        _ => None,
    };

    let start_time = match payload.start_time {
        Some(ref t) if !t.is_empty() => match NaiveTime::parse_from_str(t, "%H:%M") {
            Ok(nt) => Some(nt),
            Err(_) => {
                return response::error(
                    StatusCode::BAD_REQUEST,
                    400,
                    "开始时间格式无效，应为 HH:mm",
                );
            }
        },
        _ => None,
    };

    let end_time = match payload.end_time {
        Some(ref t) if !t.is_empty() => match NaiveTime::parse_from_str(t, "%H:%M") {
            Ok(nt) => Some(nt),
            Err(_) => {
                return response::error(
                    StatusCode::BAD_REQUEST,
                    400,
                    "结束时间格式无效，应为 HH:mm",
                );
            }
        },
        _ => None,
    };

    let location = payload.location.as_deref().map(|l| l.trim()).filter(|l| !l.is_empty());

    match db::update_exam_fields(
        pool,
        id,
        name,
        exam_date,
        start_time,
        end_time,
        location,
        payload.is_completed,
    )
    .await
    {
        Ok(exam) => {
            let updated_at = exam.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            response::ok(
                StatusCode::OK,
                200,
                "考试更新成功",
                ExamData {
                    id: exam.id,
                    name: exam.name,
                    exam_date: exam.exam_date.format("%Y-%m-%d").to_string(),
                    start_time: exam.start_time.format("%H:%M").to_string(),
                    end_time: exam.end_time.map(|t| t.format("%H:%M").to_string()),
                    location: exam.location,
                    is_completed: exam.is_completed,
                    created_at: None,
                    updated_at: Some(updated_at),
                },
            )
        }
        Err(e) => {
            tracing::error!("更新考试失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

/// DELETE /api/exams/{id} — 删除考试
pub async fn delete_exam(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    // 验证考试存在且属于当前用户
    let exam = match db::find_exam_by_id(pool, id).await {
        Ok(Some(exam)) => exam,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "考试不存在或无权操作");
        }
        Err(e) => {
            tracing::error!("查询考试失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    if exam.user_id != user_id {
        return response::error(StatusCode::NOT_FOUND, 404, "考试不存在或无权操作");
    }

    match db::delete_exam(pool, id).await {
        Ok(_) => response::ok(StatusCode::OK, 200, "考试删除成功", serde_json::Value::Null),
        Err(e) => {
            tracing::error!("删除考试失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}
