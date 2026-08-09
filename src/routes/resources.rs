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
