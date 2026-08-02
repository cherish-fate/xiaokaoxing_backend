use axum::Router;

use crate::{routes, state::AppState};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(routes::router())
        .with_state(state)
}
