use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    routes::resources::{ResourceListItem, ResourceListResponse, map_resource_item},
    state::AppState,
};

#[derive(Deserialize)]
pub struct PrepHomeQuery {
    pub category: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct RecommendationItem {
    pub id: i32,
    pub emoji: String,
    pub description: String,
    pub suggestion: String,
    pub target_type: String,
    pub target_id: i32,
}

#[derive(Serialize)]
pub struct PrepHomeData {
    pub recommendations: Vec<RecommendationItem>,
    pub resources: ResourceListResponse,
}

/// GET /api/prep/home — 获取备考中心聚合数据
pub async fn get_prep_home(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(params): Query<PrepHomeQuery>,
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

    // ---- 生成推荐项 ----
    let recommendations = build_recommendations(pool, user_id, &user.school_name, user.major_id)
        .await;

    // ---- 资源列表（默认分类 真题试卷）----
    let category = params
        .category
        .as_deref()
        .map(|c| c.trim())
        .filter(|c| !c.is_empty())
        .unwrap_or("真题试卷");
    let limit = params.limit.unwrap_or(10).max(1);
    let offset = params.offset.unwrap_or(0).max(0);

    let total = match db::count_resources_list(pool, &user.school_name, user.major_id, Some(category))
        .await
    {
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
        Some(category),
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
        PrepHomeData {
            recommendations,
            resources: ResourceListResponse { total, list },
        },
    )
}

/// 根据用户数据生成推荐项（基于规则，无 AI 依赖）
async fn build_recommendations(
    pool: &sqlx::MySqlPool,
    user_id: i32,
    school_name: &str,
    major_id: i32,
) -> Vec<RecommendationItem> {
    let mut items: Vec<RecommendationItem> = Vec::new();

    // 1. 考试倒计时推荐
    if let Ok(Some(exam)) = db::find_upcoming_exam(pool, user_id).await {
        let today = chrono::Local::now().naive_local().date();
        let days = (exam.exam_date - today).num_days();
        let description = if days == 0 {
            format!("今天有《{}》考试", exam.name)
        } else {
            format!("《{}》考试倒计时 {} 天", exam.name, days)
        };
        items.push(RecommendationItem {
            id: items.len() as i32 + 1,
            emoji: "📌".to_string(),
            description,
            suggestion: "建议：做一套模拟卷摸底 →".to_string(),
            target_type: "ai_feature".to_string(),
            target_id: 0,
        });
    }

    // 2. 今日待完成任务推荐
    if let Ok(tasks) = db::find_today_tasks(pool, user_id).await {
        let pending = tasks.iter().filter(|t| !t.is_completed).count();
        if pending > 0 {
            items.push(RecommendationItem {
                id: items.len() as i32 + 1,
                emoji: "📝".to_string(),
                description: format!("今日还有 {} 个任务待完成", pending),
                suggestion: "建议：先完成今日学习计划 →".to_string(),
                target_type: "ai_feature".to_string(),
                target_id: 0,
            });
        }
    }

    // 3. 热门资源推荐
    if let Ok(Some(resource)) = db::find_hot_resource(pool, school_name, major_id).await {
        items.push(RecommendationItem {
            id: items.len() as i32 + 1,
            emoji: "🔥".to_string(),
            description: format!("热门资源《{}》", resource.title),
            suggestion: "建议：查看这份高频资料 →".to_string(),
            target_type: "resource".to_string(),
            target_id: resource.id,
        });
    }

    items
}
