use axum::{Json, extract::State, http::StatusCode};

use serde::{Deserialize, Serialize};

use crate::{auth, db, response, state::AppState, utils::is_valid_email};

/// 注册请求体
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub nickname: String,
    pub email: String,
    pub password: String,
    pub school_name: String,
    pub major_id: i32,
}

/// 注册成功响应中的用户数据
#[derive(Serialize)]
pub struct RegisterData {
    pub id: i32,
    pub nickname: String,
    pub email: String,
    pub school_name: String,
    pub major_id: i32,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

/// POST /api/users/register — 用户注册
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "数据库未连接",
        );
    };

    let nickname = payload.nickname.trim();
    let email = payload.email.trim();
    let password = payload.password; // 密码不做 trim，允许前后空格
    let school_name = payload.school_name.trim();
    let major_id = payload.major_id;

    // ---- 参数校验 ----
    if nickname.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "昵称不能为空");
    }
    if email.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "邮箱不能为空");
    }
    if !is_valid_email(email) {
        return response::error(StatusCode::BAD_REQUEST, 400, "邮箱格式不正确");
    }
    if password.len() < 6 {
        return response::error(StatusCode::BAD_REQUEST, 400, "密码至少6位");
    }
    if school_name.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "学校名称不能为空");
    }

    // ---- 校验 major_id 是否存在 ----
    match db::find_major_by_id(pool, major_id).await {
        Ok(None) => {
            return response::error(StatusCode::BAD_REQUEST, 400, "专业ID无效");
        }
        Ok(Some(_)) => {}
        Err(e) => {
            tracing::error!("查询专业失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    }

    // ---- 检查邮箱是否已注册 ----
    match db::find_user_by_email(pool, email).await {
        Ok(Some(_)) => {
            return response::error(
                StatusCode::CONFLICT,
                409,
                "该邮箱已注册，请直接登录",
            );
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("查询用户失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    }

    // ---- 加密密码 ----
    let password_hash = match auth::hash_password(&password) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!("密码加密失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // ---- 创建用户 ----
    match db::create_user(pool, nickname, email, &password_hash, school_name, major_id).await {
        Ok(user) => {
            let created_at = user.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            response::ok(
                StatusCode::CREATED,
                201,
                "注册成功",
                RegisterData {
                    id: user.id,
                    nickname: user.nickname,
                    email: user.email,
                    school_name: user.school_name,
                    major_id: user.major_id,
                    avatar_url: user.avatar_url,
                    created_at,
                },
            )
        }
        Err(e) => {
            tracing::error!("创建用户失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}
