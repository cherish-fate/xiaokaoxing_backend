use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::{auth, db, response, state::AppState, utils::is_valid_email};

/// 登录请求体
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// 登录响应数据（扁平结构，含用户基本信息与 JWT Token）
#[derive(Serialize)]
pub struct LoginData {
    pub id: i32,
    pub nickname: String,
    pub email: String,
    pub school_name: String,
    pub major_id: i32,
    pub avatar_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub token: String,
}

/// POST /api/users/login — 用户登录
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "服务器内部错误，请稍后重试",
        );
    };

    let email = payload.email.trim();
    let password = payload.password; // 密码不做 trim，允许前后空格

    // ---- 参数校验 ----
    if email.is_empty() || password.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "邮箱和密码不能为空");
    }
    if !is_valid_email(email) {
        return response::error(StatusCode::BAD_REQUEST, 400, "邮箱格式无效");
    }

    // ---- 查找用户 ----
    let user = match db::find_user_by_email(pool, email).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return response::error(
                StatusCode::NOT_FOUND,
                404,
                "该邮箱未注册，请先注册",
            );
        }
        Err(e) => {
            tracing::error!("查询用户失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // ---- 验证密码 ----
    let valid = match auth::verify_password(&password, &user.password_hash) {
        Ok(valid) => valid,
        Err(e) => {
            tracing::error!("密码验证失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    if !valid {
        return response::error(
            StatusCode::UNAUTHORIZED,
            401,
            "密码错误，请重新输入",
        );
    }

    // ---- 生成 JWT Token ----
    let token = match auth::create_token(
        user.id,
        &user.email,
        &state.config.jwt_secret,
        state.config.jwt_expires_seconds,
    ) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("生成 Token 失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let created_at = user.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let updated_at = user.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    response::ok(
        StatusCode::OK,
        200,
        "登录成功",
        LoginData {
            id: user.id,
            nickname: user.nickname,
            email: user.email,
            school_name: user.school_name,
            major_id: user.major_id,
            avatar_url: user.avatar_url,
            created_at,
            updated_at,
            token,
        },
    )
}
