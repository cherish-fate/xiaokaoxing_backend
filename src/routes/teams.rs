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
    routes::community::calc_checkin_rate,
    state::AppState,
};

// ============ 小队列表 ============

#[derive(Deserialize)]
pub struct TeamListQuery {
    pub subject: Option<String>,
}

#[derive(Serialize)]
pub struct MyTeamItem {
    pub id: i32,
    pub name: String,
    pub subject: String,
    pub description: Option<String>,
    pub member_count: i64,
    pub max_members: i32,
    pub checkin_rate: f64,
    pub total_checkins: i64,
    pub online_count: i64,
    pub is_creator: bool,
    pub pending_requests: i64,
}

#[derive(Serialize)]
pub struct RecommendedTeamItem {
    pub id: i32,
    pub name: String,
    pub subject: String,
    pub description: Option<String>,
    pub member_count: i64,
    pub max_members: i32,
    pub checkin_rate: f64,
    pub total_checkins: i64,
    pub online_count: i64,
    pub is_creator: bool,
    pub join_status: Option<String>,
}

#[derive(Serialize)]
pub struct TeamListData {
    pub my_teams: Vec<MyTeamItem>,
    pub recommended_teams: Vec<RecommendedTeamItem>,
}

/// GET /api/teams — 获取我的小队 + 推荐小队
pub async fn list_teams(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(params): Query<TeamListQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let subject = params
        .subject
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    // 我的小队
    let my_teams = match db::find_my_teams(pool, user_id, subject).await {
        Ok(teams) => teams
            .into_iter()
            .map(|t| MyTeamItem {
                id: t.id,
                name: t.name,
                subject: t.subject,
                description: t.description,
                member_count: t.member_count,
                max_members: t.max_members,
                checkin_rate: calc_checkin_rate(t.online_count, t.member_count),
                total_checkins: t.total_checkins,
                online_count: t.online_count,
                is_creator: t.creator_id == user_id,
                pending_requests: t.pending_requests,
            })
            .collect(),
        Err(e) => {
            tracing::error!("查询我的小队失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 推荐小队（按打卡率、总打卡数排序）
    let recommended_teams = match db::find_recommended_teams(pool, user_id, subject).await {
        Ok(teams) => {
            let mut items: Vec<RecommendedTeamItem> = teams
                .into_iter()
                .map(|t| RecommendedTeamItem {
                    id: t.id,
                    name: t.name,
                    subject: t.subject,
                    description: t.description,
                    member_count: t.member_count,
                    max_members: t.max_members,
                    checkin_rate: calc_checkin_rate(t.online_count, t.member_count),
                    total_checkins: t.total_checkins,
                    online_count: t.online_count,
                    is_creator: t.creator_id == user_id,
                    join_status: t.join_status,
                })
                .collect();
            items.sort_by(|a, b| {
                b.checkin_rate
                    .partial_cmp(&a.checkin_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.total_checkins.cmp(&a.total_checkins))
            });
            items
        }
        Err(e) => {
            tracing::error!("查询推荐小队失败: {}", e);
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
        TeamListData {
            my_teams,
            recommended_teams,
        },
    )
}

// ============ 创建小队 ============

#[derive(Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub subject: String,
    pub description: Option<String>,
    pub need_approval: Option<bool>,
}

#[derive(Serialize)]
pub struct CreateTeamData {
    pub id: i32,
    pub name: String,
    pub subject: String,
    pub description: Option<String>,
    pub member_count: i32,
    pub max_members: i32,
    pub need_approval: bool,
    pub created_at: String,
}

/// POST /api/teams — 创建备考小队
pub async fn create_team(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreateTeamRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let name = payload.name.trim();
    let subject = payload.subject.trim();
    let description = payload
        .description
        .as_deref()
        .map(|d| d.trim())
        .filter(|d| !d.is_empty());

    if name.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "小队名称不能为空");
    }
    if name.chars().count() > 20 {
        return response::error(StatusCode::BAD_REQUEST, 400, "小队名称最多20字");
    }
    if subject.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "关联科目不能为空");
    }
    if let Some(d) = description {
        if d.chars().count() > 100 {
            return response::error(StatusCode::BAD_REQUEST, 400, "小队简介最多100字");
        }
    }

    let need_approval = payload.need_approval.unwrap_or(true);

    let team = match db::create_team(
        pool,
        name,
        subject,
        description,
        need_approval,
        user_id,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("创建小队失败: {}", e);
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
        "小队创建成功",
        CreateTeamData {
            id: team.id,
            name: team.name,
            subject: team.subject,
            description: team.description,
            member_count: team.member_count,
            max_members: team.max_members,
            need_approval: team.need_approval,
            created_at: team.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        },
    )
}

// ============ 小队详情 ============

#[derive(Serialize)]
pub struct TeamMemberItem {
    pub user_id: i32,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub joined_at: String,
}

#[derive(Serialize)]
pub struct TeamDetailData {
    pub id: i32,
    pub name: String,
    pub subject: String,
    pub description: Option<String>,
    pub creator_id: i32,
    pub member_count: i64,
    pub max_members: i32,
    pub checkin_rate: f64,
    pub total_checkins: i64,
    pub need_approval: bool,
    pub is_creator: bool,
    pub is_member: bool,
    pub members: Vec<TeamMemberItem>,
}

/// GET /api/teams/{id} — 获取小队详情
pub async fn get_team_detail(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let team = match db::find_team_detail(pool, id, user_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "小队不存在"),
        Err(e) => {
            tracing::error!("查询小队详情失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let members = match db::find_team_members(pool, id).await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("查询小队成员失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let members: Vec<TeamMemberItem> = members
        .into_iter()
        .map(|m| TeamMemberItem {
            user_id: m.user_id,
            nickname: m.nickname,
            avatar_url: m.avatar_url,
            role: m.role,
            joined_at: m.joined_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        })
        .collect();

    response::ok(
        StatusCode::OK,
        200,
        "success",
        TeamDetailData {
            id: team.id,
            name: team.name,
            subject: team.subject,
            description: team.description,
            creator_id: team.creator_id,
            member_count: team.member_count,
            max_members: team.max_members,
            checkin_rate: calc_checkin_rate(team.online_count, team.member_count),
            total_checkins: team.total_checkins,
            need_approval: team.need_approval,
            is_creator: team.creator_id == user_id,
            is_member: team.is_member != 0,
            members,
        },
    )
}

// ============ 申请加入小队 ============

#[derive(Serialize)]
pub struct JoinData {
    pub team_id: i32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_at: Option<String>,
}

/// POST /api/teams/{id}/join — 申请加入小队
pub async fn join_team(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let team = match db::find_team_detail(pool, id, user_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "小队不存在"),
        Err(e) => {
            tracing::error!("查询小队失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 已是成员
    if team.is_member != 0 {
        return response::error(StatusCode::CONFLICT, 409, "您已加入该小队");
    }

    // 满员检查
    if team.member_count >= team.max_members as i64 {
        return response::error(StatusCode::BAD_REQUEST, 400, "小队已满员");
    }

    // 无需审核，直接加入
    if !team.need_approval {
        match db::join_team_directly(pool, id, user_id).await {
            Ok(()) => {
                return response::ok(
                    StatusCode::OK,
                    200,
                    "已加入小队",
                    JoinData {
                        team_id: id,
                        status: "已加入".to_string(),
                        applied_at: None,
                    },
                );
            }
            Err(e) => {
                if db::is_duplicate_key_anyhow(&e) {
                    return response::error(StatusCode::CONFLICT, 409, "您已加入该小队");
                }
                tracing::error!("加入小队失败: {}", e);
                return response::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "服务器内部错误，请稍后重试",
                );
            }
        }
    }

    // 需审核：处理已有申请
    match db::find_join_request(pool, id, user_id).await {
        Ok(Some(req)) => match req.status.as_str() {
            "待审核" => {
                return response::error(
                    StatusCode::CONFLICT,
                    409,
                    "您已申请加入该小队，请等待审核",
                );
            }
            "已通过" => {
                return response::error(StatusCode::CONFLICT, 409, "您已加入该小队");
            }
            _ => {
                // 已拒绝 → 重新申请
                match db::reset_join_request_to_pending(pool, id, user_id).await {
                    Ok(req) => {
                        return response::ok(
                            StatusCode::CREATED,
                            201,
                            "申请已提交，等待队长审核",
                            JoinData {
                                team_id: id,
                                status: "待审核".to_string(),
                                applied_at: Some(
                                    req.applied_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                                ),
                            },
                        );
                    }
                    Err(e) => {
                        tracing::error!("重新申请失败: {}", e);
                        return response::error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            500,
                            "服务器内部错误，请稍后重试",
                        );
                    }
                }
            }
        },
        Ok(None) => {}
        Err(e) => {
            tracing::error!("查询入队申请失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    }

    // 创建新申请
    match db::create_join_request(pool, id, user_id).await {
        Ok(req) => response::ok(
            StatusCode::CREATED,
            201,
            "申请已提交，等待队长审核",
            JoinData {
                team_id: id,
                status: "待审核".to_string(),
                applied_at: Some(req.applied_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            },
        ),
        Err(e) => {
            if db::is_duplicate_key_anyhow(&e) {
                return response::error(
                    StatusCode::CONFLICT,
                    409,
                    "您已申请加入该小队，请等待审核",
                );
            }
            tracing::error!("创建入队申请失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 入队申请列表 ============

#[derive(Serialize)]
pub struct ApplicationItem {
    pub application_id: i32,
    pub user_id: i32,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub applied_at: String,
}

#[derive(Serialize)]
pub struct ApplicationListData {
    pub pending: Vec<ApplicationItem>,
    pub total: i64,
}

/// GET /api/teams/{id}/applications — 获取入队申请列表（仅队长）
pub async fn list_applications(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let team = match db::find_team_detail(pool, id, user_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "小队不存在"),
        Err(e) => {
            tracing::error!("查询小队失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    if team.creator_id != user_id {
        return response::error(StatusCode::FORBIDDEN, 403, "仅队长可查看申请列表");
    }

    let pending: Vec<ApplicationItem> = match db::find_pending_join_requests(pool, id).await {
        Ok(list) => list
            .into_iter()
            .map(|r| ApplicationItem {
                application_id: r.id,
                user_id: r.user_id,
                nickname: r.nickname,
                avatar_url: r.avatar_url,
                applied_at: r.applied_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            })
            .collect(),
        Err(e) => {
            tracing::error!("查询待审核申请失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let total = pending.len() as i64;

    response::ok(
        StatusCode::OK,
        200,
        "success",
        ApplicationListData { pending, total },
    )
}

// ============ 处理入队申请 ============

#[derive(Deserialize)]
pub struct ProcessApplicationRequest {
    pub action: String,
}

#[derive(Serialize)]
pub struct ProcessApplicationData {
    pub application_id: i32,
    pub status: String,
}

/// PUT /api/teams/{id}/applications/{application_id} — 处理入队申请
pub async fn process_application(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path((id, application_id)): Path<(i32, i32)>,
    Json(payload): Json<ProcessApplicationRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let action = payload.action.trim();
    let approved = match action {
        "approve" => true,
        "reject" => false,
        _ => return response::error(StatusCode::BAD_REQUEST, 400, "操作类型无效，仅支持 approve/reject"),
    };

    let team = match db::find_team_detail(pool, id, user_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "小队不存在"),
        Err(e) => {
            tracing::error!("查询小队失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    if team.creator_id != user_id {
        return response::error(StatusCode::FORBIDDEN, 403, "仅队长可处理申请");
    }

    let req = match db::find_join_request_by_id(pool, application_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "申请记录不存在"),
        Err(e) => {
            tracing::error!("查询申请记录失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    if req.team_id != id {
        return response::error(StatusCode::NOT_FOUND, 404, "申请记录不存在");
    }
    if req.status != "待审核" {
        return response::error(StatusCode::CONFLICT, 409, "该申请已处理");
    }

    if approved {
        // 满员检查
        if team.member_count >= team.max_members as i64 {
            return response::error(StatusCode::BAD_REQUEST, 400, "小队已满员，无法通过申请");
        }
        match db::approve_join_request(pool, application_id, id, req.user_id).await {
            Ok(()) => response::ok(
                StatusCode::OK,
                200,
                "已通过审核，用户已加入小队",
                ProcessApplicationData {
                    application_id,
                    status: "已通过".to_string(),
                },
            ),
            Err(e) => {
                if db::is_duplicate_key_anyhow(&e) {
                    return response::error(StatusCode::CONFLICT, 409, "该用户已是小队成员");
                }
                tracing::error!("通过申请失败: {}", e);
                response::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "服务器内部错误，请稍后重试",
                )
            }
        }
    } else {
        match db::reject_join_request(pool, application_id).await {
            Ok(()) => response::ok(
                StatusCode::OK,
                200,
                "已拒绝申请",
                ProcessApplicationData {
                    application_id,
                    status: "已拒绝".to_string(),
                },
            ),
            Err(e) => {
                tracing::error!("拒绝申请失败: {}", e);
                response::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "服务器内部错误，请稍后重试",
                )
            }
        }
    }
}

// ============ 退出小队 ============

/// DELETE /api/teams/{id}/members — 退出小队
pub async fn leave_team(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let member = match db::find_team_member(pool, id, user_id).await {
        Ok(Some(m)) => m,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "您不是该小队成员"),
        Err(e) => {
            tracing::error!("查询小队成员失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 队长不能直接退出
    if member.role == "队长" {
        return response::error(
            StatusCode::FORBIDDEN,
            403,
            "队长不能直接退出，请先解散小队",
        );
    }

    match db::leave_team(pool, id, user_id).await {
        Ok(()) => response::ok(StatusCode::OK, 200, "已退出小队", ()),
        Err(e) => {
            tracing::error!("退出小队失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 解散小队 ============

/// DELETE /api/teams/{id} — 解散小队（仅队长）
pub async fn dissolve_team(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let team = match db::find_team_detail(pool, id, user_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "小队不存在"),
        Err(e) => {
            tracing::error!("查询小队失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    if team.creator_id != user_id {
        return response::error(StatusCode::FORBIDDEN, 403, "仅队长可解散小队");
    }

    match db::dissolve_team(pool, id).await {
        Ok(()) => response::ok(StatusCode::OK, 200, "小队已解散", ()),
        Err(e) => {
            tracing::error!("解散小队失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}


// ============ 我的小队 ============

#[derive(Serialize)]
pub struct MyTeamListItem {
    pub id: i32,
    pub name: String,
    pub subject: String,
    pub role: String,
    pub member_count: i64,
    pub max_members: i32,
    pub checkin_rate: f64,
}

#[derive(Serialize)]
pub struct MyTeamsData {
    pub list: Vec<MyTeamListItem>,
}

/// GET /api/teams/my-teams — 获取当前用户已加入的学习小队
pub async fn get_my_teams(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(params): Query<TeamListQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let subject = params
        .subject
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    let teams = match db::find_my_teams(pool, user_id, subject).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("查询我的小队失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let list = teams
        .into_iter()
        .map(|t| MyTeamListItem {
            id: t.id,
            name: t.name,
            subject: t.subject,
            role: if t.creator_id == user_id {
                "队长".to_string()
            } else {
                "成员".to_string()
            },
            member_count: t.member_count,
            max_members: t.max_members,
            checkin_rate: calc_checkin_rate(t.online_count, t.member_count),
        })
        .collect();

    response::ok(StatusCode::OK, 200, "success", MyTeamsData { list })
}
