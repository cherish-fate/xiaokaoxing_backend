use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::http::{HeaderMap, StatusCode, header::AUTHORIZATION};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64, // user_id
    pub email: String,
    pub exp: usize,
}

#[derive(Debug, Clone)]
pub struct AuthorizedUser {
    pub user_id: i64,
    pub email: String,
    pub token: String,
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("密码加密失败: {e}"))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| anyhow!("密码哈希解析失败: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn create_token(
    user_id: i64,
    email: &str,
    secret: &str,
    expires_in_seconds: u64,
) -> Result<String> {
    let exp = SystemTime::now()
        .checked_add(Duration::from_secs(expires_in_seconds))
        .context("计算过期时间失败")?
        .duration_since(UNIX_EPOCH)
        .context("计算时间戳失败")?
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        exp,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .context("JWT 编码失败")?;

    Ok(token)
}

pub fn decode_token(token: &str, secret: &str) -> Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .context("JWT 解码失败")?;
    Ok(data.claims)
}

pub async fn require_user(
    headers: &HeaderMap,
    state: &AppState,
) -> std::result::Result<AuthorizedUser, (StatusCode, String)> {
    let header_value = headers
        .get(AUTHORIZATION)
        .ok_or((StatusCode::UNAUTHORIZED, "未登录或 Token 缺失".to_string()))?;
    let auth_value = header_value
        .to_str()
        .map_err(|_| (StatusCode::UNAUTHORIZED, "未登录或 Token 缺失".to_string()))?;
    let token = auth_value
        .strip_prefix("Bearer ")
        .ok_or((StatusCode::UNAUTHORIZED, "未登录或 Token 缺失".to_string()))?
        .trim();

    let claims = decode_token(token, &state.config.jwt_secret).map_err(|_| {
        (
            StatusCode::FORBIDDEN,
            "Token 已过期，请重新登录".to_string(),
        )
    })?;

    Ok(AuthorizedUser {
        user_id: claims.sub,
        email: claims.email,
        token: token.to_string(),
    })
}
