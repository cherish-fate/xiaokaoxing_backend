use axum::{extract::State, http::StatusCode};
use serde::Serialize;

use crate::{db, response, state::AppState};

/// 专业列表项
#[derive(Serialize)]
pub struct MajorItem {
    pub id: i32,
    pub name: String,
}

/// GET /api/majors — 获取所有专业列表
pub async fn list_majors(State(state): State<AppState>) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "数据库未连接",
        );
    };

    match db::find_all_majors(pool).await {
        Ok(majors) => {
            let items: Vec<MajorItem> = majors
                .into_iter()
                .map(|m| MajorItem { id: m.id, name: m.name })
                .collect();
            response::ok(StatusCode::OK, 200, "success", items)
        }
        Err(e) => {
            tracing::error!("查询专业列表失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}
