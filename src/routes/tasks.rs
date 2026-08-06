use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
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
pub struct UpdateTaskRequest {
    pub is_completed: bool,
}

#[derive(Serialize)]
pub struct TaskData {
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

/// PUT /api/tasks/{id} — 更新任务状态
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

    match db::update_task_status(pool, id, payload.is_completed).await {
        Ok(task) => {
            let updated_at = task.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            response::ok(
                StatusCode::OK,
                200,
                "任务状态更新成功",
                TaskData {
                    id: task.id,
                    task_name: task.task_name,
                    is_completed: task.is_completed,
                    updated_at: Some(updated_at),
                },
            )
        }
        Err(e) => {
            tracing::error!("更新任务状态失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}
