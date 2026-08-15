use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    state::AppState,
};

#[derive(Serialize)]
pub struct NotificationSettingsData {
    pub exam_reminder: bool,
    pub checkin_reminder: bool,
}

/// GET /api/settings/notifications — 获取通知设置
pub async fn get_notifications(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    match db::get_user_settings(pool, user_id).await {
        Ok(s) => response::ok(
            StatusCode::OK,
            200,
            "success",
            NotificationSettingsData {
                exam_reminder: s.exam_reminder,
                checkin_reminder: s.checkin_reminder,
            },
        ),
        Err(e) => {
            tracing::error!("查询通知设置失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateNotificationsRequest {
    pub exam_reminder: Option<bool>,
    pub checkin_reminder: Option<bool>,
}

/// PUT /api/settings/notifications — 更新通知设置
pub async fn update_notifications(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<UpdateNotificationsRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let current = match db::get_user_settings(pool, user_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("查询通知设置失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let exam_reminder = payload.exam_reminder.unwrap_or(current.exam_reminder);
    let checkin_reminder = payload.checkin_reminder.unwrap_or(current.checkin_reminder);

    match db::update_user_settings(pool, user_id, exam_reminder, checkin_reminder).await {
        Ok(s) => response::ok(
            StatusCode::OK,
            200,
            "通知设置已更新",
            NotificationSettingsData {
                exam_reminder: s.exam_reminder,
                checkin_reminder: s.checkin_reminder,
            },
        ),
        Err(e) => {
            tracing::error!("更新通知设置失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

#[derive(Serialize)]
pub struct AboutData {
    pub version: String,
    pub app_name: String,
    pub description: String,
}

/// GET /api/settings/about — 获取关于信息
pub async fn about() -> axum::response::Response {
    response::ok(
        StatusCode::OK,
        200,
        "success",
        AboutData {
            version: "v1.0.0".to_string(),
            app_name: "校考星".to_string(),
            description: "大学生期末备考一站式平台".to_string(),
        },
    )
}
