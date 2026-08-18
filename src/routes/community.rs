use axum::{extract::State, http::StatusCode};
use chrono::Datelike;
use serde::Serialize;

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    routes::checkin::compute_user_checkin_stats,
    state::AppState,
};

#[derive(Serialize)]
pub struct CheckinStats {
    pub total_days: i64,
    pub continuous_days: i32,
    pub rank: i64,
}

#[derive(Serialize)]
pub struct HotVoteItem {
    pub id: i32,
    pub subject: String,
    pub title: String,
    pub vote_count: i32,
    pub confidence: f64,
    pub stars: i32,
}

#[derive(Serialize)]
pub struct HotTeamItem {
    pub id: i32,
    pub name: String,
    pub subject: String,
    pub member_count: i64,
    pub max_members: i32,
    pub checkin_rate: f64,
    pub total_checkins: i64,
    pub online_count: i64,
    pub is_creator: bool,
    pub join_status: Option<String>,
}

#[derive(Serialize)]
pub struct LatestResourceItem {
    pub id: i32,
    pub title: String,
    pub author: Option<String>,
    pub created_at: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct CommunityHomeData {
    pub total_users: i64,
    pub checked_week_days: Vec<u32>,
    pub checkin_stats: CheckinStats,
    pub hot_votes: Vec<HotVoteItem>,
    pub hot_teams: Vec<HotTeamItem>,
    pub latest_resources: Vec<LatestResourceItem>,
}

/// 根据票数推导星级（1-5）
pub fn stars_from_vote_count(vote_count: i32) -> i32 {
    if vote_count >= 200 {
        5
    } else if vote_count >= 100 {
        4
    } else if vote_count >= 50 {
        3
    } else if vote_count >= 10 {
        2
    } else {
        1
    }
}

/// 计算打卡率（今日打卡人数 / 总成员数 × 100%，保留两位小数）
pub fn calc_checkin_rate(online_count: i64, member_count: i64) -> f64 {
    if member_count <= 0 {
        0.0
    } else {
        let rate = (online_count as f64 / member_count as f64) * 100.0;
        (rate * 100.0).round() / 100.0
    }
}

/// 计算置信度（vote_count / 总票数 × 100，保留两位小数）
pub fn calc_confidence(vote_count: i32, total: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        let c = (vote_count as f64 / total as f64) * 100.0;
        (c * 100.0).round() / 100.0
    }
}

/// GET /api/community/home — 获取社区主页聚合数据
pub async fn get_home(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

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

    // 0. 平台用户总数
    let total_users = match db::count_total_users(pool).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("统计用户总数失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 0. 本周已打卡日期索引（0=周一 ... 6=周日）
    let today = chrono::Local::now().naive_local().date();
    let monday = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
    let week_dates = match db::find_checkin_dates_between(
        pool,
        user_id,
        monday,
        monday + chrono::Duration::days(6),
    )
    .await
    {
        Ok(dates) => dates,
        Err(e) => {
            tracing::error!("查询本周打卡日期失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let checked_week_days: Vec<u32> = week_dates
        .iter()
        .map(|d| d.weekday().num_days_from_monday())
        .collect();

    // 1. 打卡统计
    let checkin_stats = match compute_user_checkin_stats(pool, user_id, &user.school_name).await {
        Ok(s) => CheckinStats {
            total_days: s.total_days,
            continuous_days: s.continuous_days,
            rank: s.rank,
        },
        Err(e) => {
            tracing::error!("计算打卡统计失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 2. 本周高频考点 TOP3
    let hot_votes = match db::find_passed_votes(pool, user_id, None).await {
        Ok(votes) => {
            let total = db::sum_passed_votes_count(pool).await.unwrap_or(0);
            votes
                .into_iter()
                .take(3)
                .map(|v| HotVoteItem {
                    id: v.id,
                    subject: v.subject,
                    title: v.title,
                    stars: stars_from_vote_count(v.vote_count),
                    vote_count: v.vote_count,
                    confidence: calc_confidence(v.vote_count, total),
                })
                .collect()
        }
        Err(e) => {
            tracing::error!("查询热门考点失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 3. 热门备考小队（最多3个，按打卡率、总打卡数排序）
    let hot_teams = match db::find_hot_teams(pool, user_id).await {
        Ok(teams) => {
            let mut items: Vec<HotTeamItem> = teams
                .into_iter()
                .map(|t| {
                    let checkin_rate = calc_checkin_rate(t.online_count, t.member_count);
                    HotTeamItem {
                        id: t.id,
                        name: t.name,
                        subject: t.subject,
                        member_count: t.member_count,
                        max_members: t.max_members,
                        checkin_rate,
                        total_checkins: t.total_checkins,
                        online_count: t.online_count,
                        is_creator: t.creator_id == user_id,
                        join_status: t.join_status,
                    }
                })
                .collect();
            // 按打卡率降序、总打卡数降序排序后取前3
            items.sort_by(|a, b| {
                b.checkin_rate
                    .partial_cmp(&a.checkin_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.total_checkins.cmp(&a.total_checkins))
            });
            items.truncate(3);
            items
        }
        Err(e) => {
            tracing::error!("查询热门小队失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 4. 本校最新共享资料（已审核，最多5条）
    let latest_resources = match db::find_latest_resources_by_school(pool, &user.school_name, 5).await {
        Ok(resources) => resources
            .into_iter()
            .map(|r| LatestResourceItem {
                id: r.id,
                title: r.title,
                author: r.author,
                created_at: r.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                status: "已上线".to_string(),
            })
            .collect(),
        Err(e) => {
            tracing::error!("查询最新资料失败: {}", e);
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
        "success",
        CommunityHomeData {
            total_users,
            checked_week_days,
            checkin_stats,
            hot_votes,
            hot_teams,
            latest_resources,
        },
    )
}
