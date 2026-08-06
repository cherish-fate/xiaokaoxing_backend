use axum::{
    extract::{Query, State},
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
