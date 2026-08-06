use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::Response,
};

use crate::{response, state::AppState};

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
