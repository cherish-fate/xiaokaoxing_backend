use axum::{Json, response::IntoResponse};
use serde_json::json;

use crate::state::AppState;
use axum::extract::State;

pub async fn api_index() -> impl IntoResponse {
    Json(json!({
        "code": 200,
        "message": "ok",
        "data": {
            "name": "xiaokaoxing-backend",
            "version": "0.1.0",
            "status": "running",
            "api_prefix": "/api"
        }
    }))
}

pub async fn index() -> impl IntoResponse {
    Json(json!({
        "name": "校考星后端",
        "version": "0.1.0",
        "status": "running"
    }))
}

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let db_status = if state.db.is_some() {
        "connected"
    } else {
        "disconnected"
    };

    Json(json!({
        "status": "ok",
        "database": db_status,
        "database_name": state.database_name,
    }))
}
