use axum::Router;
use tower_http::services::ServeDir;

use crate::{routes, state::AppState};

pub fn build_router(state: AppState) -> Router {
    let upload_dir = state.config.upload_dir.clone();
    Router::new()
        .merge(routes::router())
        // 上传文件静态访问
        .nest_service("/uploads", ServeDir::new(upload_dir))
        .with_state(state)
}
