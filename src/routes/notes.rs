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

fn parse_tags(raw: &Option<String>) -> Vec<String> {
    raw.as_deref()
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
}

fn validate_tags(tags: &[String]) -> Option<String> {
    if tags.len() > 3 {
        return Some("标签最多3个".to_string());
    }
    for tag in tags {
        let t = tag.trim();
        if t.is_empty() {
            return Some("标签不能为空字符串".to_string());
        }
        if t.chars().count() > 20 {
            return Some("单个标签最多20个字符".to_string());
        }
    }
    None
}

fn content_preview(content: &Option<String>) -> String {
    let Some(raw) = content else {
        return String::new();
    };
    let mut text = String::new();
    for line in raw.lines() {
        let t = line.trim_start_matches(|c| matches!(c, '#' | '*' | '>' | '-' | '`'));
        text.push_str(t.trim());
        text.push(' ');
    }
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = joined.chars();
    let head: String = chars.by_ref().take(120).collect();
    if chars.next().is_some() {
        format!("{}...", head)
    } else {
        head
    }
}

// ============ 笔记列表 ============

#[derive(Deserialize)]
pub struct NoteListQuery {
    pub tag: Option<String>,
    pub keyword: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct NoteListItem {
    pub id: i32,
    pub title: String,
    pub content_preview: String,
    pub tags: Vec<String>,
    pub is_pinned: bool,
    pub source_type: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct NoteListData {
    pub total: i64,
    pub list: Vec<NoteListItem>,
}

/// GET /api/notes
pub async fn list_notes(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(query): Query<NoteListQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let tag = query.tag.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let keyword = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let total = match db::count_notes(pool, user_id, tag, keyword).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("统计笔记数量失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let notes = match db::find_notes(pool, user_id, tag, keyword, limit, offset).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("查询笔记列表失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let list: Vec<NoteListItem> = notes
        .into_iter()
        .map(|n| NoteListItem {
            id: n.id,
            title: n.title,
            content_preview: content_preview(&n.content),
            tags: parse_tags(&n.tags),
            is_pinned: n.is_pinned,
            source_type: n.source_type,
            updated_at: n.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        })
        .collect();
    response::ok(
        StatusCode::OK,
        200,
        "success",
        NoteListData { total, list },
    )
}

// ============ 创建笔记 ============

#[derive(Deserialize)]
pub struct CreateNoteRequest {
    pub title: String,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct NoteCreatedData {
    pub id: i32,
    pub title: String,
    pub content: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
}

/// POST /api/notes
pub async fn create_note(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreateNoteRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let title = payload.title.trim();
    if title.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "笔记标题不能为空");
    }
    if title.chars().count() > 50 {
        return response::error(StatusCode::BAD_REQUEST, 400, "笔记标题最多50个字符");
    }
    let tags = payload.tags.unwrap_or_default();
    if let Some(msg) = validate_tags(&tags) {
        return response::error(StatusCode::BAD_REQUEST, 400, msg);
    }
    let content = payload
        .content
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let tags_json = serde_json::to_string(&tags).ok();

    match db::create_note(pool, user_id, title, content, tags_json.as_deref(), "manual", None)
        .await
    {
        Ok(note) => response::ok(
            StatusCode::CREATED,
            201,
            "笔记创建成功",
            NoteCreatedData {
                id: note.id,
                title: note.title,
                content: note.content,
                tags: parse_tags(&note.tags),
                created_at: note.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            },
        ),
        Err(e) => {
            tracing::error!("创建笔记失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 笔记详情 ============

#[derive(Serialize)]
pub struct NoteDetailData {
    pub id: i32,
    pub title: String,
    pub content: Option<String>,
    pub tags: Vec<String>,
    pub is_pinned: bool,
    pub source_type: String,
    pub source_id: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

/// GET /api/notes/{id}
pub async fn get_note(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let note = match db::find_note_by_id(pool, id).await {
        Ok(Some(n)) => n,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "笔记不存在");
        }
        Err(e) => {
            tracing::error!("查询笔记失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    if note.user_id != user_id {
        return response::error(StatusCode::NOT_FOUND, 404, "笔记不存在");
    }
    response::ok(
        StatusCode::OK,
        200,
        "success",
        NoteDetailData {
            id: note.id,
            title: note.title,
            content: note.content,
            tags: parse_tags(&note.tags),
            is_pinned: note.is_pinned,
            source_type: note.source_type,
            source_id: note.source_id,
            created_at: note.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            updated_at: note.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        },
    )
}

// ============ 更新笔记 ============

#[derive(Deserialize)]
pub struct UpdateNoteRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct NoteUpdatedData {
    pub id: i32,
    pub updated_at: String,
}

/// PUT /api/notes/{id}
pub async fn update_note(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateNoteRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let existing = match db::find_note_by_id(pool, id).await {
        Ok(Some(n)) => n,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "笔记不存在");
        }
        Err(e) => {
            tracing::error!("查询笔记失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    if existing.user_id != user_id {
        return response::error(StatusCode::NOT_FOUND, 404, "笔记不存在");
    }
    if payload.title.is_none() && payload.content.is_none() && payload.tags.is_none() {
        return response::error(StatusCode::BAD_REQUEST, 400, "至少需要更新一个字段");
    }

    let title = match payload.title {
        Some(t) => {
            let trimmed = t.trim();
            if trimmed.is_empty() {
                return response::error(StatusCode::BAD_REQUEST, 400, "笔记标题不能为空");
            }
            if trimmed.chars().count() > 50 {
                return response::error(
                    StatusCode::BAD_REQUEST,
                    400,
                    "笔记标题最多50个字符",
                );
            }
            trimmed.to_string()
        }
        None => existing.title.clone(),
    };
    let content = payload.content.map(|c| c.trim().to_string());
    let tags_json = match payload.tags {
        Some(tags) => {
            if let Some(msg) = validate_tags(&tags) {
                return response::error(StatusCode::BAD_REQUEST, 400, msg);
            }
            serde_json::to_string(&tags).ok()
        }
        None => existing.tags.clone(),
    };

    match db::update_note(
        pool,
        id,
        user_id,
        &title,
        content.as_deref(),
        tags_json.as_deref(),
        existing.is_pinned,
    )
    .await
    {
        Ok(Some(note)) => response::ok(
            StatusCode::OK,
            200,
            "笔记更新成功",
            NoteUpdatedData {
                id: note.id,
                updated_at: note.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            },
        ),
        Ok(None) => response::error(StatusCode::NOT_FOUND, 404, "笔记不存在"),
        Err(e) => {
            tracing::error!("更新笔记失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 删除笔记 ============

/// DELETE /api/notes/{id}
pub async fn delete_note(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    match db::delete_note(pool, id, user_id).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            "笔记删除成功",
            serde_json::Value::Null,
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "笔记不存在"),
        Err(e) => {
            tracing::error!("删除笔记失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 摘录笔记 ============

#[derive(Deserialize)]
pub struct ExcerptRequest {
    pub source_type: String,
    pub source_id: i32,
    pub title: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct ExcerptCreatedData {
    pub id: i32,
    pub title: String,
    pub source_type: String,
    pub source_id: i32,
    pub created_at: String,
}

/// POST /api/notes/excerpt
pub async fn create_excerpt(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<ExcerptRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let source_type = payload.source_type.trim();
    if !matches!(source_type, "resource" | "note" | "question") {
        return response::error(
            StatusCode::BAD_REQUEST,
            400,
            "来源类型仅支持：resource/note/question",
        );
    }
    let title = payload.title.trim();
    if title.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "笔记标题不能为空");
    }
    if title.chars().count() > 50 {
        return response::error(StatusCode::BAD_REQUEST, 400, "笔记标题最多50个字符");
    }
    let content = payload.content.trim();
    if content.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "摘录内容不能为空");
    }
    let tags = payload.tags.unwrap_or_default();
    if let Some(msg) = validate_tags(&tags) {
        return response::error(StatusCode::BAD_REQUEST, 400, msg);
    }

    let source_exists = match source_type {
        "resource" => db::resource_exists(pool, payload.source_id).await,
        "note" => db::find_note_by_id(pool, payload.source_id)
            .await
            .map(|n| n.is_some()),
        _ => db::find_daily_question_by_id(pool, payload.source_id)
            .await
            .map(|q| q.is_some()),
    };
    match source_exists {
        Ok(true) => {}
        Ok(false) => {
            return response::error(StatusCode::NOT_FOUND, 404, "来源资源不存在");
        }
        Err(e) => {
            tracing::error!("校验来源资源失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    }

    let tags_json = serde_json::to_string(&tags).ok();
    match db::create_note(
        pool,
        user_id,
        title,
        Some(content),
        tags_json.as_deref(),
        source_type,
        Some(payload.source_id),
    )
    .await
    {
        Ok(note) => response::ok(
            StatusCode::CREATED,
            201,
            "摘录成功，已创建笔记",
            ExcerptCreatedData {
                id: note.id,
                title: note.title,
                source_type: note.source_type,
                source_id: payload.source_id,
                created_at: note.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            },
        ),
        Err(e) => {
            tracing::error!("创建摘录笔记失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 切换置顶 ============

#[derive(Deserialize)]
pub struct PinRequest {
    pub is_pinned: bool,
}

#[derive(Serialize)]
pub struct PinData {
    pub id: i32,
    pub is_pinned: bool,
}

/// PUT /api/notes/{id}/pin
pub async fn toggle_pin(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
    Json(payload): Json<PinRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let existing = match db::find_note_by_id(pool, id).await {
        Ok(Some(n)) => n,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "笔记不存在");
        }
        Err(e) => {
            tracing::error!("查询笔记失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    if existing.user_id != user_id {
        return response::error(StatusCode::NOT_FOUND, 404, "笔记不存在");
    }
    match db::update_note(
        pool,
        id,
        user_id,
        &existing.title,
        existing.content.as_deref(),
        existing.tags.as_deref(),
        payload.is_pinned,
    )
    .await
    {
        Ok(Some(note)) => response::ok(
            StatusCode::OK,
            200,
            "置顶状态已更新",
            PinData {
                id: note.id,
                is_pinned: note.is_pinned,
            },
        ),
        Ok(None) => response::error(StatusCode::NOT_FOUND, 404, "笔记不存在"),
        Err(e) => {
            tracing::error!("更新置顶状态失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}
