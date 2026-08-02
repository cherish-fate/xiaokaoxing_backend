use axum::{
    Router,
    routing::{get, post},
};

pub mod auth;
pub mod health;

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .route("/", get(health::index))
        .route("/health", get(health::health))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
}
