use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    state::AppState,
};

fn format_size(bytes: i64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let value = bytes as f64;
    if value >= GB {
        format!("{:.1} GB", value / GB)
    } else if value >= MB {
        format!("{:.0} MB", value / MB)
    } else if value >= KB {
        format!("{:.0} KB", value / KB)
    } else {
        format!("{} B", bytes)
    }
}

// ============ 文档列表 ============

#[derive(Deserialize)]
pub struct DocumentListQuery {
    pub category: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct DocumentItem {
    pub id: i32,
    pub name: String,
    pub file_url: String,
    pub file_size: i64,
    pub file_type: String,
    pub category: Option<String>,
    pub is_offline: bool,
    pub last_opened_at: Option<String>,
}

#[derive(Serialize)]
pub struct DocumentListData {
    pub total: i64,
    pub list: Vec<DocumentItem>,
}

fn document_to_item(doc: db::Document) -> DocumentItem {
    DocumentItem {
        id: doc.id,
        name: doc.name,
        file_url: doc.file_url,
        file_size: doc.file_size,
        file_type: doc.file_type,
        category: doc.category,
        is_offline: doc.is_offline,
        last_opened_at: doc
            .last_opened_at
            .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
    }
}

/// GET /api/documents
pub async fn list_documents(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(query): Query<DocumentListQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let category = query
        .category
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let total = match db::count_documents(pool, user_id, category).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("统计文档数量失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let documents = match db::find_documents(pool, user_id, category, limit, offset).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("查询文档列表失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let list: Vec<DocumentItem> = documents.into_iter().map(document_to_item).collect();
    response::ok(
        StatusCode::OK,
        200,
        "success",
        DocumentListData { total, list },
    )
}

// ============ 存储状态 ============

#[derive(Serialize)]
pub struct StorageData {
    pub used: i64,
    pub used_display: String,
    pub total: i64,
    pub total_display: String,
    pub percentage: f64,
    pub is_warning: bool,
}

/// GET /api/documents/storage
pub async fn get_storage(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let used = match db::sum_document_sizes(pool, user_id).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("统计文档存储大小失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    const TOTAL: i64 = 1024 * 1024 * 1024;
    let percentage = if TOTAL > 0 {
        (used as f64 * 100.0 / TOTAL as f64 * 10.0).round() / 10.0
    } else {
        0.0
    };
    response::ok(
        StatusCode::OK,
        200,
        "success",
        StorageData {
            used,
            used_display: format_size(used),
            total: TOTAL,
            total_display: format_size(TOTAL),
            percentage,
            is_warning: percentage >= 90.0,
        },
    )
}

// ============ 离线状态 ============

#[derive(Deserialize)]
pub struct OfflineRequest {
    pub is_offline: bool,
}

#[derive(Serialize)]
pub struct OfflineData {
    pub id: i32,
    pub is_offline: bool,
}

/// PUT /api/documents/{id}/offline
pub async fn update_offline(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
    Json(payload): Json<OfflineRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    match db::update_document_offline(pool, id, user_id, payload.is_offline).await {
        Ok(Some(doc)) => response::ok(
            StatusCode::OK,
            200,
            "离线状态已更新",
            OfflineData {
                id: doc.id,
                is_offline: doc.is_offline,
            },
        ),
        Ok(None) => response::error(StatusCode::NOT_FOUND, 404, "文档不存在"),
        Err(e) => {
            tracing::error!("更新文档离线状态失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 删除文档 ============

/// DELETE /api/documents/{id}
pub async fn delete_document(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    match db::delete_document(pool, id, user_id).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            "文档已删除",
            serde_json::Value::Null,
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "文档不存在"),
        Err(e) => {
            tracing::error!("删除文档失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}
