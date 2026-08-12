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
    routes::community::{calc_confidence, stars_from_vote_count},
    state::AppState,
};

// ============ 投票列表 ============

#[derive(Deserialize)]
pub struct VoteListQuery {
    pub subject: Option<String>,
}

#[derive(Serialize)]
pub struct VoteItem {
    pub id: i32,
    pub subject: String,
    pub title: String,
    pub description: Option<String>,
    pub vote_count: i32,
    pub confidence: f64,
    pub stars: i32,
    pub has_voted: bool,
}

#[derive(Serialize)]
pub struct VoteListData {
    pub list: Vec<VoteItem>,
}

/// GET /api/votes — 获取投票列表（仅已通过，按票数降序）
pub async fn list_votes(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(params): Query<VoteListQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let subject = params
        .subject
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    let votes = match db::find_passed_votes(pool, user_id, subject).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("查询投票列表失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let total = db::sum_passed_votes_count(pool).await.unwrap_or(0);

    let list: Vec<VoteItem> = votes
        .into_iter()
        .map(|v| VoteItem {
            id: v.id,
            subject: v.subject,
            title: v.title,
            description: v.description,
            vote_count: v.vote_count,
            confidence: calc_confidence(v.vote_count, total),
            stars: stars_from_vote_count(v.vote_count),
            has_voted: v.has_voted != 0,
        })
        .collect();

    response::ok(StatusCode::OK, 200, "success", VoteListData { list })
}

// ============ 提交考点 ============

#[derive(Deserialize)]
pub struct CreateVoteRequest {
    pub subject: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct CreateVoteData {
    pub id: i32,
    pub subject: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
}

/// POST /api/votes — 提交新考点（待审核）
pub async fn create_vote(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreateVoteRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let subject = payload.subject.trim();
    let title = payload.title.trim();
    let description = payload
        .description
        .as_deref()
        .map(|d| d.trim())
        .filter(|d| !d.is_empty());

    if subject.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "科目不能为空");
    }
    if title.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "考点名称不能为空");
    }
    if title.chars().count() > 30 {
        return response::error(StatusCode::BAD_REQUEST, 400, "考点名称最多30字");
    }
    if let Some(d) = description {
        if d.chars().count() > 200 {
            return response::error(StatusCode::BAD_REQUEST, 400, "补充说明最多200字");
        }
    }

    let vote = match db::create_vote(pool, subject, title, description, user_id).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("提交考点失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    response::ok(
        StatusCode::CREATED,
        201,
        "考点提交成功，等待管理员审核",
        CreateVoteData {
            id: vote.id,
            subject: vote.subject,
            title: vote.title,
            status: vote.status,
            created_at: vote.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        },
    )
}

// ============ 投票 ============

#[derive(Serialize)]
pub struct CastVoteData {
    pub vote_id: i32,
    pub vote_count: i32,
    pub confidence: f64,
}

/// POST /api/votes/{id}/vote — 对考点投票
pub async fn cast_vote(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let vote = match db::find_vote_by_id(pool, id).await {
        Ok(Some(v)) => v,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "考点不存在"),
        Err(e) => {
            tracing::error!("查询考点失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    if vote.status != "已通过" {
        return response::error(StatusCode::BAD_REQUEST, 400, "该考点未上线，无法投票");
    }

    match db::cast_vote(pool, id, user_id).await {
        Ok(()) => {
            let total = db::sum_passed_votes_count(pool).await.unwrap_or(0);
            // 重新查询最新票数
            let updated = match db::find_vote_by_id(pool, id).await {
                Ok(Some(v)) => v.vote_count,
                _ => vote.vote_count + 1,
            };
            response::ok(
                StatusCode::OK,
                200,
                "投票成功",
                CastVoteData {
                    vote_id: id,
                    vote_count: updated,
                    confidence: calc_confidence(updated, total),
                },
            )
        }
        Err(e) => {
            if db::is_duplicate_key_anyhow(&e) {
                return response::error(
                    StatusCode::CONFLICT,
                    409,
                    "您已投过该考点，不可重复投票",
                );
            }
            tracing::error!("投票失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 我的投票记录 ============

#[derive(Serialize)]
pub struct MyVoteItem {
    pub vote_id: i32,
    pub subject: String,
    pub title: String,
    pub vote_count: i32,
    pub confidence: f64,
    pub voted_at: String,
}

#[derive(Serialize)]
pub struct MyVotesData {
    pub list: Vec<MyVoteItem>,
}

/// GET /api/votes/my-votes — 获取我的投票记录
pub async fn get_my_votes(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let records = match db::find_my_votes(pool, user_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("查询我的投票记录失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let total = db::sum_passed_votes_count(pool).await.unwrap_or(0);

    let list: Vec<MyVoteItem> = records
        .into_iter()
        .map(|r| MyVoteItem {
            vote_id: r.vote_id,
            subject: r.subject,
            title: r.title,
            confidence: calc_confidence(r.vote_count, total),
            vote_count: r.vote_count,
            voted_at: r.voted_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        })
        .collect();

    response::ok(StatusCode::OK, 200, "success", MyVotesData { list })
}

// ============ 我提交的考点 ============

#[derive(Serialize)]
pub struct MySubmissionItem {
    pub id: i32,
    pub subject: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct MySubmissionsData {
    pub list: Vec<MySubmissionItem>,
}

/// GET /api/votes/my-submissions — 获取我提交的考点
pub async fn get_my_submissions(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let submissions = match db::find_my_submissions(pool, user_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("查询我提交的考点失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let list: Vec<MySubmissionItem> = submissions
        .into_iter()
        .map(|v| MySubmissionItem {
            id: v.id,
            subject: v.subject,
            title: v.title,
            description: v.description,
            status: v.status,
            created_at: v.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        })
        .collect();

    response::ok(
        StatusCode::OK,
        200,
        "success",
        MySubmissionsData { list },
    )
}
