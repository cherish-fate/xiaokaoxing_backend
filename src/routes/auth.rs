use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth,
    db,
    response,
    state::AppState,
    utils::is_valid_email,
};

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub email: String,
}

#[derive(Serialize)]
pub struct AuthData {
    pub token: String,
    #[serde(rename = "tokenType")]
    pub token_type: &'static str,
    #[serde(rename = "expiresIn")]
    pub expires_in: u64,
    pub user: UserInfo,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "数据库未连接",
            None,
        )
        .into_response();
    };

    let email = payload.email.trim();
    let password = payload.password.trim();

    if !is_valid_email(email) {
        return response::error(
            StatusCode::BAD_REQUEST,
            400,
            "邮箱格式不正确",
            None,
        )
        .into_response();
    }

    if password.len() < 6 {
        return response::error(
            StatusCode::BAD_REQUEST,
            400,
            "密码长度至少为6位",
            None,
        )
        .into_response();
    }

    match db::find_user_by_email(pool, email).await {
        Ok(Some(_)) => {
            return response::error(
                StatusCode::CONFLICT,
                409,
                "邮箱已被注册",
                None,
            )
            .into_response();
        }
        Ok(None) => {}
        Err(e) => {
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                e.to_string(),
                None,
            )
            .into_response();
        }
    }

    let password_hash = match auth::hash_password(password) {
        Ok(hash) => hash,
        Err(e) => {
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                e.to_string(),
                None,
            )
            .into_response();
        }
    };

    let user_id = match db::create_user(pool, email, &password_hash).await {
        Ok(id) => id,
        Err(e) => {
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                e.to_string(),
                None,
            )
            .into_response();
        }
    };

    let token = match auth::create_token(
        user_id,
        email,
        &state.config.jwt_secret,
        state.config.jwt_expires_seconds,
    ) {
        Ok(token) => token,
        Err(e) => {
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                e.to_string(),
                None,
            )
            .into_response();
        }
    };

    response::ok(
        "注册成功",
        AuthData {
            token,
            token_type: "Bearer",
            expires_in: state.config.jwt_expires_seconds,
            user: UserInfo {
                id: user_id,
                email: email.to_string(),
            },
        },
    )
    .into_response()
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "数据库未连接",
            None,
        )
        .into_response();
    };

    let email = payload.email.trim();
    let password = payload.password.trim();

    if !is_valid_email(email) {
        return response::error(
            StatusCode::BAD_REQUEST,
            400,
            "邮箱格式不正确",
            None,
        )
        .into_response();
    }

    if password.is_empty() {
        return response::error(
            StatusCode::BAD_REQUEST,
            400,
            "密码不能为空",
            None,
        )
        .into_response();
    }

    let user = match db::find_user_by_email(pool, email).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return response::error(
                StatusCode::UNAUTHORIZED,
                401,
                "邮箱或密码错误",
                None,
            )
            .into_response();
        }
        Err(e) => {
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                e.to_string(),
                None,
            )
            .into_response();
        }
    };

    let valid = match auth::verify_password(password, &user.password_hash) {
        Ok(valid) => valid,
        Err(e) => {
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                e.to_string(),
                None,
            )
            .into_response();
        }
    };

    if !valid {
        return response::error(
            StatusCode::UNAUTHORIZED,
            401,
            "邮箱或密码错误",
            None,
        )
        .into_response();
    }

    let token = match auth::create_token(
        user.id,
        &user.email,
        &state.config.jwt_secret,
        state.config.jwt_expires_seconds,
    ) {
        Ok(token) => token,
        Err(e) => {
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                e.to_string(),
                None,
            )
            .into_response();
        }
    };

    response::ok(
        "登录成功",
        AuthData {
            token,
            token_type: "Bearer",
            expires_in: state.config.jwt_expires_seconds,
            user: UserInfo {
                id: user.id,
                email: user.email,
            },
        },
    )
    .into_response()
}
