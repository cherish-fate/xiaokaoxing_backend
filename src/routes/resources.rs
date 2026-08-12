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

#[derive(Deserialize)]
pub struct ResourceQuery {
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct ResourceItem {
    pub id: i32,
    pub title: String,
    pub type_tag: String,
    pub description: Option<String>,
    pub file_url: String,
}

#[derive(Serialize)]
pub struct ResourceListData {
    pub total: i64,
    pub list: Vec<ResourceItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_hint: Option<String>,
}

/// GET /api/resources/recommended — 获取推荐资源
pub async fn get_recommended_resources(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(params): Query<ResourceQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    // 查询用户信息获取学校和专业
    let user = match db::find_user_by_id(pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "用户不存在");
        }
        Err(e) => {
            tracing::error!("查询用户失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let limit = params.limit.unwrap_or(4);

    let resources = match db::find_recommended_resources(
        pool,
        &user.school_name,
        user.major_id,
        limit,
    )
    .await
    {
        Ok(resources) => resources,
        Err(e) => {
            tracing::error!("查询推荐资源失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let total = resources.len() as i64;
    let list: Vec<ResourceItem> = resources
        .into_iter()
        .map(|r| ResourceItem {
            id: r.id,
            title: r.title,
            type_tag: r.type_tag,
            description: r.description,
            file_url: r.file_url,
        })
        .collect();

    let empty_hint = if total > 0 {
        None
    } else {
        Some("暂无推荐资源".to_string())
    };

    response::ok(
        StatusCode::OK,
        200,
        "success",
        ResourceListData {
            total,
            list,
            empty_hint,
        },
    )
}

// ============ 备考中心资源接口 ============

#[derive(Deserialize)]
pub struct ResourceListQuery {
    pub category: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct ResourceListItem {
    pub id: i32,
    pub title: String,
    pub category: String,
    pub type_tag: String,
    pub is_hot: bool,
    pub author: Option<String>,
    pub created_at: String,
    pub view_count: i64,
    pub is_favorited: bool,
    pub file_url: String,
}

#[derive(Serialize)]
pub struct ResourceListResponse {
    pub total: i64,
    pub list: Vec<ResourceListItem>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub keyword: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub total: i64,
    pub keyword: String,
    pub list: Vec<ResourceListItem>,
}

#[derive(Deserialize)]
pub struct FavoriteRequest {
    pub is_favorited: bool,
}

#[derive(Serialize)]
pub struct FavoriteData {
    pub resource_id: i32,
    pub is_favorited: bool,
}

pub fn map_resource_item(r: db::ResourceWithFavorite) -> ResourceListItem {
    ResourceListItem {
        id: r.id,
        title: r.title,
        category: r.category,
        type_tag: r.type_tag,
        is_hot: r.is_hot,
        author: r.author,
        created_at: r.created_at.format("%Y-%m-%d").to_string(),
        view_count: r.view_count,
        is_favorited: r.is_favorited != 0,
        file_url: r.file_url,
    }
}

/// GET /api/resources — 获取资源列表（分类筛选）
pub async fn list_resources(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(params): Query<ResourceListQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let user = match db::find_user_by_id(pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "用户不存在");
        }
        Err(e) => {
            tracing::error!("查询用户失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let category = params
        .category
        .as_deref()
        .map(|c| c.trim())
        .filter(|c| !c.is_empty());
    let limit = params.limit.unwrap_or(10).max(1);
    let offset = params.offset.unwrap_or(0).max(0);

    let total = match db::count_resources_list(pool, &user.school_name, user.major_id, category).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("统计资源总数失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let resources = match db::find_resources_list(
        pool,
        user_id,
        &user.school_name,
        user.major_id,
        category,
        limit,
        offset,
    )
    .await
    {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("查询资源列表失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let list: Vec<ResourceListItem> = resources.into_iter().map(map_resource_item).collect();

    response::ok(
        StatusCode::OK,
        200,
        "success",
        ResourceListResponse { total, list },
    )
}

/// GET /api/resources/search — 搜索资源
pub async fn search_resources(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(params): Query<SearchQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let keyword = match params.keyword.as_deref().map(|k| k.trim()) {
        Some(k) if !k.is_empty() => k,
        _ => {
            return response::error(StatusCode::BAD_REQUEST, 400, "关键词不能为空");
        }
    };
    if keyword.chars().count() > 50 {
        return response::error(StatusCode::BAD_REQUEST, 400, "关键词最多50个字符");
    }

    let user = match db::find_user_by_id(pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "用户不存在");
        }
        Err(e) => {
            tracing::error!("查询用户失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let limit = params.limit.unwrap_or(10).max(1);
    let offset = params.offset.unwrap_or(0).max(0);

    let total = match db::count_search_resources(pool, &user.school_name, user.major_id, keyword).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("统计搜索结果失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let resources = match db::search_resources(
        pool,
        user_id,
        &user.school_name,
        user.major_id,
        keyword,
        limit,
        offset,
    )
    .await
    {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("搜索资源失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let list: Vec<ResourceListItem> = resources.into_iter().map(map_resource_item).collect();

    response::ok(
        StatusCode::OK,
        200,
        "success",
        SearchResponse {
            total,
            keyword: keyword.to_string(),
            list,
        },
    )
}

/// POST /api/resources/{id}/favorite — 收藏/取消收藏
pub async fn toggle_favorite(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
    Json(payload): Json<FavoriteRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    // 校验资源是否存在
    match db::resource_exists(pool, id).await {
        Ok(false) => {
            return response::error(StatusCode::NOT_FOUND, 404, "资源不存在");
        }
        Ok(true) => {}
        Err(e) => {
            tracing::error!("查询资源失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    }

    if payload.is_favorited {
        match db::create_favorite(pool, user_id, id).await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("添加收藏失败: {}", e);
                return response::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "服务器内部错误，请稍后重试",
                );
            }
        }
        response::ok(
            StatusCode::OK,
            200,
            "收藏成功",
            FavoriteData {
                resource_id: id,
                is_favorited: true,
            },
        )
    } else {
        match db::delete_favorite(pool, user_id, id).await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("取消收藏失败: {}", e);
                return response::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "服务器内部错误，请稍后重试",
                );
            }
        }
        response::ok(
            StatusCode::OK,
            200,
            "已取消收藏",
            FavoriteData {
                resource_id: id,
                is_favorited: false,
            },
        )
    }
}

// ============ 共享资料上传 ============

/// 允许的资源文件扩展名
fn allowed_resource_extension(filename: &str) -> Option<String> {
    let ext = filename.rsplit('.').next()?.to_lowercase();
    match ext.as_str() {
        "pdf" | "doc" | "docx" | "ppt" | "pptx" | "zip" => Some(ext),
        _ => None,
    }
}

/// 校验资料分类是否合法
fn is_valid_category(category: &str) -> bool {
    matches!(
        category,
        "真题试卷" | "复习提纲" | "课件考点" | "自测题库"
    )
}

#[derive(Serialize)]
pub struct UploadResourceData {
    pub id: i32,
    pub title: String,
    pub status: String,
    pub created_at: String,
}

/// POST /api/resources — 上传共享资料（multipart/form-data）
pub async fn upload_resource(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    mut multipart: Multipart,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let mut fields: HashMap<String, String> = HashMap::new();
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_ext: Option<String> = None;
    let mut original_name = String::from("file");

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            original_name = field.file_name().unwrap_or("file").to_string();
            let ext = match allowed_resource_extension(&original_name) {
                Some(e) => e,
                None => {
                    return response::error(
                        StatusCode::BAD_REQUEST,
                        400,
                        "不支持的文件格式，仅支持PDF/Word/PPT/ZIP",
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
            // 校验文件大小（最大 50MB）
            const MAX_SIZE: usize = 50 * 1024 * 1024;
            if bytes.len() > MAX_SIZE {
                return response::error(StatusCode::BAD_REQUEST, 400, "文件大小超过50MB限制");
            }
            file_ext = Some(ext);
            file_bytes = Some(bytes.to_vec());
        } else {
            // 文本字段
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
        None => return response::error(StatusCode::BAD_REQUEST, 400, "未找到上传文件"),
    };
    let ext = file_ext.unwrap();

    let title = fields
        .get("title")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let title = match title {
        Some(t) => {
            if t.chars().count() > 50 {
                return response::error(StatusCode::BAD_REQUEST, 400, "资料标题最多50字");
            }
            t
        }
        None => return response::error(StatusCode::BAD_REQUEST, 400, "资料标题不能为空"),
    };

    let subject = fields
        .get("subject")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let subject = match subject {
        Some(s) => Some(s),
        None => return response::error(StatusCode::BAD_REQUEST, 400, "科目不能为空"),
    };

    // type 与 category 一致，任一缺失或非法均报错
    let category = fields
        .get("category")
        .or_else(|| fields.get("type"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let category = match category {
        Some(c) if is_valid_category(&c) => c,
        Some(_) => {
            return response::error(
                StatusCode::BAD_REQUEST,
                400,
                "资料分类无效，仅支持 真题试卷/复习提纲/课件考点/自测题库",
            );
        }
        None => return response::error(StatusCode::BAD_REQUEST, 400, "资料分类不能为空"),
    };

    let description = fields
        .get("description")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(d) = description.as_ref() {
        if d.chars().count() > 200 {
            return response::error(StatusCode::BAD_REQUEST, 400, "资料简介最多200字");
        }
    }

    // 查询用户信息（学校 + 昵称）
    let user = match db::find_user_by_id(pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "用户不存在"),
        Err(e) => {
            tracing::error!("查询用户失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 保存文件
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
        tracing::error!("保存文件失败: {}", e);
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "服务器内部错误，请稍后重试",
        );
    }

    let relative = format!("uploads/{}", stored_name);
    let file_url = state.config.public_url(&relative);
    let author = user.nickname.clone();

    let resource_id = match db::create_resource(
        pool,
        user_id,
        &title,
        subject.as_deref(),
        &category,
        "本校专属",
        Some(&user.school_name),
        Some(&author),
        description.as_deref(),
        &file_url,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("创建资源失败: {}", e);
            // 清理已保存的文件
            let _ = tokio::fs::remove_file(&file_path).await;
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let created_at = match db::find_resource_created_at(pool, resource_id).await {
        Ok(Some(t)) => t.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        _ => chrono::Local::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };

    let _ = original_name; // 原始文件名仅用于扩展名校验

    response::ok(
        StatusCode::CREATED,
        201,
        "资料提交成功，等待管理员审核（1-2工作日）",
        UploadResourceData {
            id: resource_id,
            title,
            status: "审核中".to_string(),
            created_at,
        },
    )
}

// ============ 我的上传记录 ============

#[derive(Serialize)]
pub struct MyUploadItem {
    pub id: i32,
    pub title: String,
    pub category: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub status: String,
    pub created_at: String,
    pub view_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reject_reason: Option<String>,
}

#[derive(Serialize)]
pub struct MyUploadsData {
    pub list: Vec<MyUploadItem>,
}

/// GET /api/resources/my-uploads — 获取我的上传记录
pub async fn get_my_uploads(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let uploads = match db::find_my_uploads(pool, user_id).await {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("查询我的上传记录失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let list: Vec<MyUploadItem> = uploads
        .into_iter()
        .map(|r| MyUploadItem {
            id: r.id,
            title: r.title,
            type_field: r.category.clone(),
            category: r.category,
            status: r.status,
            created_at: r.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            view_count: r.view_count,
            reject_reason: r.reject_reason,
        })
        .collect();

    response::ok(StatusCode::OK, 200, "success", MyUploadsData { list })
}
