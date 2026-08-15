use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::Response,
};

use crate::{db, response, state::AppState};

pub struct AuthenticatedUser(pub i32);

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));

        let token = match auth_header {
            Some(t) if !t.is_empty() => t,
            _ => {
                return Err(response::error(
                    axum::http::StatusCode::UNAUTHORIZED,
                    401,
                    "未提供有效的认证令牌",
                ));
            }
        };

        let claims = match crate::auth::verify_token(token, &state.config.jwt_secret) {
            Ok(claims) => claims,
            Err(_) => {
                return Err(response::error(
                    axum::http::StatusCode::UNAUTHORIZED,
                    401,
                    "Token无效或已过期，请重新登录",
                ));
            }
        };

        let user_id: i32 = match claims.sub.parse() {
            Ok(id) => id,
            Err(_) => {
                return Err(response::error(
                    axum::http::StatusCode::UNAUTHORIZED,
                    401,
                    "Token无效或已过期，请重新登录",
                ));
            }
        };

        Ok(AuthenticatedUser(user_id))
    }
}

pub struct AdminUser(pub i32);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthenticatedUser(user_id) = AuthenticatedUser::from_request_parts(parts, state).await?;

        let Some(pool) = state.db.as_ref() else {
            return Err(response::error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "数据库未连接",
            ));
        };

        match db::find_user_by_id(pool, user_id).await {
            Ok(Some(user)) if user.is_admin && !user.is_disabled => Ok(AdminUser(user_id)),
            Ok(Some(_)) => Err(response::error(
                axum::http::StatusCode::FORBIDDEN,
                403,
                "无管理员权限",
            )),
            Ok(None) => Err(response::error(
                axum::http::StatusCode::UNAUTHORIZED,
                401,
                "用户不存在",
            )),
            Err(e) => {
                tracing::error!("管理员鉴权查询用户失败: {}", e);
                Err(response::error(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "服务器内部错误，请稍后重试",
                ))
            }
        }
    }
}