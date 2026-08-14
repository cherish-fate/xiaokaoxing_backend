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

const ALLOWED_COLORS: [&str; 5] = ["red", "yellow", "green", "blue", "purple"];

// ============ 书签列表 ============

#[derive(Deserialize)]
pub struct BookmarkQuery {
    pub color: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Serialize)]
pub struct BookmarkItem {
    pub id: i32,
    pub quote: String,
    pub source_title: String,
    pub source_url: Option<String>,
    pub source_type: String,
    pub source_id: Option<i32>,
    pub anchor: Option<String>,
    pub note: Option<String>,
    pub color: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct BookmarkListData {
    pub total: i64,
    pub list: Vec<BookmarkItem>,
}

fn bookmark_to_item(bookmark: db::Bookmark) -> BookmarkItem {
    BookmarkItem {
        id: bookmark.id,
        quote: bookmark.quote,
        source_title: bookmark.source_title,
        source_url: bookmark.source_url,
        source_type: bookmark.source_type,
        source_id: bookmark.source_id,
        anchor: bookmark.anchor,
        note: bookmark.note,
        color: bookmark.color.unwrap_or_else(|| "yellow".to_string()),
        created_at: bookmark.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    }
}

/// GET /api/bookmarks
pub async fn list_bookmarks(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(query): Query<BookmarkQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let color = query
        .color
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(c) = color {
        if !ALLOWED_COLORS.contains(&c) {
            return response::error(
                StatusCode::BAD_REQUEST,
                400,
                "颜色仅支持：red/yellow/green/blue/purple",
            );
        }
    }
    let keyword = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let total = match db::count_bookmarks(pool, user_id, color, keyword).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("统计书签数量失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let bookmarks = match db::find_bookmarks(pool, user_id, color, keyword).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("查询书签列表失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let list: Vec<BookmarkItem> = bookmarks.into_iter().map(bookmark_to_item).collect();
    response::ok(
        StatusCode::OK,
        200,
        "success",
        BookmarkListData { total, list },
    )
}

// ============ 创建书签 ============

#[derive(Deserialize)]
pub struct CreateBookmarkRequest {
    pub quote: String,
    pub source_title: String,
    pub source_url: Option<String>,
    pub source_type: String,
    pub source_id: Option<i32>,
    pub anchor: Option<String>,
    pub note: Option<String>,
    pub color: Option<String>,
}

#[derive(Serialize)]
pub struct BookmarkCreatedData {
    pub id: i32,
    pub color: String,
    pub created_at: String,
}

/// POST /api/bookmarks
pub async fn create_bookmark(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreateBookmarkRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let quote = payload.quote.trim();
    if quote.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "引用内容不能为空");
    }
    if quote.chars().count() > 200 {
        return response::error(StatusCode::BAD_REQUEST, 400, "引用内容最多200个字符");
    }
    let source_title = payload.source_title.trim();
    if source_title.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "来源标题不能为空");
    }
    let source_type = payload.source_type.trim();
    if !matches!(source_type, "resource" | "note" | "question") {
        return response::error(
            StatusCode::BAD_REQUEST,
            400,
            "来源类型仅支持：resource/note/question",
        );
    }
    let note = payload
        .note
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(n) = note {
        if n.chars().count() > 100 {
            return response::error(StatusCode::BAD_REQUEST, 400, "备注最多100个字符");
        }
    }
    let color = payload
        .color
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("yellow");
    if !ALLOWED_COLORS.contains(&color) {
        return response::error(
            StatusCode::BAD_REQUEST,
            400,
            "颜色仅支持：red/yellow/green/blue/purple",
        );
    }
    let source_url = payload
        .source_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let anchor = payload
        .anchor
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    match db::create_bookmark(
        pool,
        user_id,
        quote,
        source_title,
        source_url,
        source_type,
        payload.source_id,
        anchor,
        note,
        color,
    )
    .await
    {
        Ok(bookmark) => response::ok(
            StatusCode::CREATED,
            201,
            "书签添加成功",
            BookmarkCreatedData {
                id: bookmark.id,
                color: bookmark.color.unwrap_or_else(|| "yellow".to_string()),
                created_at: bookmark.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            },
        ),
        Err(e) => {
            tracing::error!("创建书签失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 更新书签 ============

#[derive(Deserialize)]
pub struct UpdateBookmarkRequest {
    pub note: Option<String>,
    pub color: Option<String>,
}

#[derive(Serialize)]
pub struct BookmarkUpdatedData {
    pub id: i32,
    pub note: Option<String>,
    pub color: String,
    pub updated_at: String,
}

/// PUT /api/bookmarks/{id}
pub async fn update_bookmark(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateBookmarkRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    if payload.note.is_none() && payload.color.is_none() {
        return response::error(StatusCode::BAD_REQUEST, 400, "至少需要更新一个字段");
    }
    let existing = match db::find_bookmark_by_id(pool, id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "书签不存在");
        }
        Err(e) => {
            tracing::error!("查询书签失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    if existing.user_id != user_id {
        return response::error(StatusCode::NOT_FOUND, 404, "书签不存在");
    }
    let note = match payload.note {
        Some(n) => {
            let trimmed = n.trim();
            if trimmed.chars().count() > 100 {
                return response::error(StatusCode::BAD_REQUEST, 400, "备注最多100个字符");
            }
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        None => existing.note.clone(),
    };
    let color = payload
        .color
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(existing.color.as_deref().unwrap_or("yellow"))
        .to_string();
    if !ALLOWED_COLORS.contains(&color.as_str()) {
        return response::error(
            StatusCode::BAD_REQUEST,
            400,
            "颜色仅支持：red/yellow/green/blue/purple",
        );
    }

    match db::update_bookmark(
        pool,
        id,
        user_id,
        &existing.quote,
        &existing.source_title,
        existing.source_url.as_deref(),
        &existing.source_type,
        existing.source_id,
        existing.anchor.as_deref(),
        note.as_deref(),
        &color,
    )
    .await
    {
        Ok(Some(bookmark)) => response::ok(
            StatusCode::OK,
            200,
            "书签更新成功",
            BookmarkUpdatedData {
                id: bookmark.id,
                note: bookmark.note.clone(),
                color: bookmark.color.unwrap_or_else(|| "yellow".to_string()),
                updated_at: bookmark.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            },
        ),
        Ok(None) => response::error(StatusCode::NOT_FOUND, 404, "书签不存在"),
        Err(e) => {
            tracing::error!("更新书签失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 删除书签 ============

/// DELETE /api/bookmarks/{id}
pub async fn delete_bookmark(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    match db::delete_bookmark(pool, id, user_id).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            "书签删除成功",
            serde_json::Value::Null,
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "书签不存在"),
        Err(e) => {
            tracing::error!("删除书签失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}
