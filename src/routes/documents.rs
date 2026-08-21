use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

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

const MAX_DOCUMENT_SIZE: usize = 50 * 1024 * 1024;

fn allowed_document_extension(filename: &str) -> Option<String> {
    let ext = filename.rsplit('.').next()?.to_lowercase();
    match ext.as_str() {
        "pdf" | "doc" | "docx" | "ppt" | "pptx" | "txt" | "md" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => Some(ext),
        _ => None,
    }
}

fn document_file_type(ext: &str) -> String {
    let t = match ext {
        "pdf" => "PDF",
        "doc" | "docx" => "Word",
        "ppt" | "pptx" => "PPT",
        "txt" | "md" => "Text",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => "Image",
        _ => "Other",
    };
    t.to_string()
}

#[derive(Serialize)]
pub struct UploadDocumentData {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub file_url: String,
    pub file_size: i64,
    pub file_type: String,
    pub category: Option<String>,
    pub is_offline: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn document_to_detail(doc: db::Document) -> UploadDocumentData {
    UploadDocumentData {
        id: doc.id,
        user_id: doc.user_id,
        name: doc.name,
        file_url: doc.file_url,
        file_size: doc.file_size,
        file_type: doc.file_type,
        category: doc.category,
        is_offline: doc.is_offline,
        last_opened_at: doc
            .last_opened_at
            .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        created_at: doc
            .created_at
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
        updated_at: doc
            .updated_at
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    }
}

/// POST /api/documents — 上传文档（multipart/form-data）
pub async fn upload_document(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    mut multipart: Multipart,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let mut fields: HashMap<String, String> = HashMap::new();
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut original_name = String::from("file");
    let mut file_ext: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            original_name = field.file_name().unwrap_or("file").to_string();
            let ext = match allowed_document_extension(&original_name) {
                Some(e) => e,
                None => {
                    return response::error(
                        StatusCode::BAD_REQUEST,
                        400,
                        "不支持的文件格式，仅支持 PDF/Word/PPT/图片/文本",
                    );
                }
            };
            let bytes = match field.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("读取文件内容失败: {}", e);
                    return response::error(StatusCode::BAD_REQUEST, 400, "文件读取失败");
                }
            };
            if bytes.len() > MAX_DOCUMENT_SIZE {
                return response::error(StatusCode::BAD_REQUEST, 400, "文件大小超过50MB限制");
            }
            file_ext = Some(ext);
            file_bytes = Some(bytes.to_vec());
        } else {
            let value = match field.text().await {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("读取表单字段失败: {}", e);
                    return response::error(StatusCode::BAD_REQUEST, 400, "表单字段读取失败");
                }
            };
            fields.insert(name, value);
        }
    }

    let file_bytes = match file_bytes {
        Some(b) => b,
        None => return response::error(StatusCode::BAD_REQUEST, 400, "请上传文件"),
    };
    let ext = file_ext.unwrap();
    let file_type = document_file_type(&ext);

    let name = fields
        .get("name")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| original_name.clone());
    if name.chars().count() > 200 {
        return response::error(StatusCode::BAD_REQUEST, 400, "文档名称不能超过200字符");
    }

    let category = fields
        .get("category")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(c) = category.as_ref() {
        if c.chars().count() > 20 {
            return response::error(StatusCode::BAD_REQUEST, 400, "分类不能超过20字符");
        }
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let stored_name = format!("{}.{}", ts, ext);
    let dir = std::path::Path::new(&state.config.upload_dir);
    if let Err(e) = tokio::fs::create_dir_all(dir).await {
        tracing::error!("创建上传目录失败: {}", e);
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "服务器内部错误，请稍后重试",
        );
    }
    let file_path = dir.join(&stored_name);
    if let Err(e) = tokio::fs::write(&file_path, &file_bytes).await {
        tracing::error!("保存文档失败: {}", e);
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "服务器内部错误，请稍后重试",
        );
    }

    let relative = format!("uploads/{}", stored_name);
    let file_url = state.config.public_url(&relative);

    let document_id = match db::create_document(
        pool,
        user_id,
        &name,
        &file_url,
        file_bytes.len() as i64,
        &file_type,
        category.as_deref(),
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("创建文档记录失败: {}", e);
            let _ = tokio::fs::remove_file(&file_path).await;
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let document = match db::find_document_by_id(pool, document_id).await {
        Ok(Some(doc)) => doc,
        Ok(None) | Err(_) => {
            let _ = tokio::fs::remove_file(&file_path).await;
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    response::ok(
        StatusCode::OK,
        200,
        "文档上传成功",
        document_to_detail(document),
    )
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

// ============ 从资源同步到文档库 ============

/// 从 URL 中提取文件扩展名（小写）
fn extract_extension(url: &str) -> Option<String> {
    let path = url.split('?').next().unwrap_or(url);
    let filename = path.rsplit('/').next()?;
    let ext = filename.rsplit('.').next()?.to_lowercase();
    if ext.is_empty() || ext == filename {
        return None;
    }
    Some(ext)
}

#[derive(Deserialize)]
pub struct FromResourceRequest {
    pub resource_id: i32,
}

#[derive(Serialize)]
pub struct FromResourceData {
    pub document_id: i32,
    pub already_exists: bool,
}

/// POST /api/documents/from-resource — 将资源同步到个人文档库
pub async fn from_resource(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<FromResourceRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    // 查询资源
    let resource = match db::find_resource_by_id(pool, payload.resource_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "资源不存在"),
        Err(e) => {
            tracing::error!("查询资源失败: {:#}", e);
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };

    // 去重：同一用户已同步过该资源（按 file_url 匹配）
    if let Ok(Some(existing)) = db::find_document_by_user_url(pool, user_id, &resource.file_url).await {
        return response::ok(
            StatusCode::OK,
            200,
            "success",
            FromResourceData {
                document_id: existing.id,
                already_exists: true,
            },
        );
    }

    // 字段映射
    let name = resource.title;
    let file_url = resource.file_url.clone();
    let category = if resource.category.is_empty() {
        Some("资源收藏".to_string())
    } else {
        Some(resource.category)
    };
    let file_type = extract_extension(&file_url)
        .map(|ext| document_file_type(&ext))
        .unwrap_or_else(|| "Other".to_string());

    let document_id = match db::create_document(
        pool,
        user_id,
        &name,
        &file_url,
        0,
        &file_type,
        category.as_deref(),
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("从资源创建文档失败: {:#}", e);
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };

    response::ok(
        StatusCode::OK,
        200,
        "success",
        FromResourceData {
            document_id,
            already_exists: false,
        },
    )
}

/// DELETE /api/documents/from-resource/{resourceId} — 取消资源收藏时移除同步的文档
pub async fn delete_from_resource(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(resource_id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    // 查询资源获取 file_url（资源不存在也视为文档不存在，统一返回 404）
    let file_url = match db::find_resource_by_id(pool, resource_id).await {
        Ok(Some(r)) => r.file_url,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "文档不存在"),
        Err(e) => {
            tracing::error!("查询资源失败: {:#}", e);
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };

    match db::delete_document_by_user_url(pool, user_id, &file_url).await {
        Ok(true) => response::ok(StatusCode::OK, 200, "success", serde_json::Value::Null),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "文档不存在"),
        Err(e) => {
            tracing::error!("按资源删除文档失败: {:#}", e);
            response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试")
        }
    }
}
