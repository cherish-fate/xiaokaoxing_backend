use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    state::AppState,
};

#[derive(Serialize)]
pub struct TaskItem {
    pub id: i32,
    pub task_name: String,
    pub plan_date: String,
    pub is_completed: bool,
}

#[derive(Serialize)]
pub struct TodayTasksData {
    pub has_tasks: bool,
    pub total: i64,
    pub completed: i64,
    pub list: Vec<TaskItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_hint: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub task_name: String,
    pub plan_date: Option<String>,
    pub is_completed: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub task_name: Option<String>,
    pub plan_date: Option<String>,
    pub is_completed: Option<bool>,
}

#[derive(Serialize)]
pub struct TaskFullData {
    pub id: i32,
    pub task_name: String,
    pub plan_date: String,
    pub is_completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Serialize)]
pub struct TaskStatusData {
    pub id: i32,
    pub task_name: String,
    pub is_completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// GET /api/tasks/today — 获取今日任务
pub async fn get_today_tasks(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let tasks = match db::find_today_tasks(pool, user_id).await {
        Ok(tasks) => tasks,
        Err(e) => {
            tracing::error!("查询今日任务失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let total = tasks.len() as i64;
    let completed = tasks.iter().filter(|t| t.is_completed).count() as i64;
    let has_tasks = total > 0;

    let list: Vec<TaskItem> = tasks
        .into_iter()
        .map(|t| TaskItem {
            id: t.id,
            task_name: t.task_name,
            plan_date: t.plan_date.format("%Y-%m-%d").to_string(),
            is_completed: t.is_completed,
        })
        .collect();

    let empty_hint = if has_tasks {
        None
    } else {
        Some("今日暂无任务，去备考页添加学习计划吧".to_string())
    };

    response::ok(
        StatusCode::OK,
        200,
        "success",
        TodayTasksData {
            has_tasks,
            total,
            completed,
            list,
            empty_hint,
        },
    )
}

/// POST /api/tasks — 添加任务
pub async fn create_task(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreateTaskRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let task_name = payload.task_name.trim();
    if task_name.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "任务名称不能为空");
    }
    if task_name.len() > 200 {
        return response::error(StatusCode::BAD_REQUEST, 400, "任务名称不能超过200字符");
    }

    let plan_date = match payload.plan_date {
        Some(ref d) if !d.is_empty() => match NaiveDate::parse_from_str(d, "%Y-%m-%d") {
            Ok(nd) => nd,
            Err(_) => {
                return response::error(
                    StatusCode::BAD_REQUEST,
                    400,
                    "计划日期格式无效，应为 YYYY-MM-DD",
                );
            }
        },
        _ => chrono::Local::now().naive_local().date(),
    };

    let is_completed = payload.is_completed.unwrap_or(false);

    match db::create_task(pool, user_id, task_name, plan_date, is_completed).await {
        Ok(task) => {
            let created_at = task.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let updated_at = task.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            response::ok(
                StatusCode::CREATED,
                201,
                "任务添加成功",
                TaskFullData {
                    id: task.id,
                    task_name: task.task_name,
                    plan_date: task.plan_date.format("%Y-%m-%d").to_string(),
                    is_completed: task.is_completed,
                    created_at: Some(created_at),
                    updated_at: Some(updated_at),
                },
            )
        }
        Err(e) => {
            tracing::error!("创建任务失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

/// PUT /api/tasks/{id} — 更新任务信息（包含状态切换）
pub async fn update_task(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateTaskRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    // 验证任务存在且属于当前用户
    let task = match db::find_task_by_id(pool, id).await {
        Ok(Some(task)) => task,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "任务不存在或无权操作");
        }
        Err(e) => {
            tracing::error!("查询任务失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    if task.user_id != user_id {
        return response::error(StatusCode::NOT_FOUND, 404, "任务不存在或无权操作");
    }

    // 判断请求体的意图：如果只传了 is_completed，则按"更新状态"处理
    // 否则按"更新任务信息"处理，需要至少一个可更新字段
    let has_other_fields = payload.task_name.is_some() || payload.plan_date.is_some();
    let has_is_completed_only = payload.is_completed.is_some() && !has_other_fields;

    if !has_is_completed_only {
        // "更新任务信息"模式：至少需要一个字段
        let task_name_empty = match &payload.task_name {
            Some(n) => n.trim().is_empty(),
            None => false,
        };
        if !has_other_fields && !payload.is_completed.is_some() {
            return response::error(StatusCode::BAD_REQUEST, 400, "至少需要更新一个字段");
        }
        if task_name_empty {
            return response::error(StatusCode::BAD_REQUEST, 400, "任务名称不能为空字符串");
        }
    }

    // 解析可选字段
    let task_name = payload
        .task_name
        .as_deref()
        .map(|n| n.trim())
        .filter(|n| !n.is_empty());

    let plan_date = match payload.plan_date {
        Some(ref d) if !d.is_empty() => match NaiveDate::parse_from_str(d, "%Y-%m-%d") {
            Ok(nd) => Some(nd),
            Err(_) => {
                return response::error(
                    StatusCode::BAD_REQUEST,
                    400,
                    "计划日期格式无效，应为 YYYY-MM-DD",
                );
            }
        },
        _ => None,
    };

    match db::update_task_fields(pool, id, task_name, plan_date, payload.is_completed).await {
        Ok(task) => {
            let updated_at = task.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();

            if has_is_completed_only {
                // 只更新状态 → 简洁响应
                response::ok(
                    StatusCode::OK,
                    200,
                    "任务状态更新成功",
                    TaskStatusData {
                        id: task.id,
                        task_name: task.task_name,
                        is_completed: task.is_completed,
                        updated_at: Some(updated_at),
                    },
                )
            } else {
                // 更新其他信息 → 完整响应
                response::ok(
                    StatusCode::OK,
                    200,
                    "任务更新成功",
                    TaskFullData {
                        id: task.id,
                        task_name: task.task_name,
                        plan_date: task.plan_date.format("%Y-%m-%d").to_string(),
                        is_completed: task.is_completed,
                        created_at: None,
                        updated_at: Some(updated_at),
                    },
                )
            }
        }
        Err(e) => {
            tracing::error!("更新任务失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

/// DELETE /api/tasks/{id} — 删除任务
pub async fn delete_task(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    // 验证任务存在且属于当前用户
    let task = match db::find_task_by_id(pool, id).await {
        Ok(Some(task)) => task,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "任务不存在或无权操作");
        }
        Err(e) => {
            tracing::error!("查询任务失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    if task.user_id != user_id {
        return response::error(StatusCode::NOT_FOUND, 404, "任务不存在或无权操作");
    }

    match db::delete_task(pool, id).await {
        Ok(_) => response::ok(StatusCode::OK, 200, "任务删除成功", serde_json::Value::Null),
        Err(e) => {
            tracing::error!("删除任务失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}
