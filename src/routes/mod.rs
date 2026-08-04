use axum::{
    Router,
    routing::{get, post},
};

pub mod auth;
pub mod health;
pub mod majors;
pub mod users;

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        // 健康检查
        .route("/", get(health::index))
        .route("/health", get(health::health))
        // 专业列表（注册页面下拉框数据源）
        .route("/api/majors", get(majors::list_majors))
        // 用户注册
        .route("/api/users/register", post(users::register))
        // 用户登录
        .route("/api/users/login", post(auth::login))
}
