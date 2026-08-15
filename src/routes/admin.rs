use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{auth, auth_ext::AdminUser, db, response, state::AppState};

#[derive(Deserialize)]
pub struct AdminLoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AdminLoginData {
    pub id: i32,
    pub nickname: String,
    pub email: String,
    pub is_admin: bool,
    pub token: String,
}

/// POST /api/admin/login — 管理员登录
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<AdminLoginRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let email = payload.email.trim();
    let password = payload.password;

    if email.is_empty() || password.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "邮箱和密码不能为空");
    }

    let user = match db::find_user_by_email(pool, email).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return response::error(StatusCode::UNAUTHORIZED, 401, "管理员账号不存在");
        }
        Err(e) => {
            tracing::error!("管理员登录查询用户失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    if !user.is_admin {
        return response::error(StatusCode::FORBIDDEN, 403, "该账号不是管理员");
    }
    if user.is_disabled {
        return response::error(StatusCode::FORBIDDEN, 403, "账号已被禁用，请联系管理员");
    }

    let valid = match auth::verify_password(&password, &user.password_hash) {
        Ok(valid) => valid,
        Err(e) => {
            tracing::error!("管理员密码验证失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    if !valid {
        return response::error(StatusCode::UNAUTHORIZED, 401, "管理员密码错误");
    }

    let token = match auth::create_token(
        user.id,
        &user.email,
        &state.config.jwt_secret,
        state.config.jwt_expires_seconds,
    ) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("生成管理员 Token 失败: {}", e);
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
        "管理员登录成功",
        AdminLoginData {
            id: user.id,
            nickname: user.nickname,
            email: user.email,
            is_admin: user.is_admin,
            token,
        },
    )
}

#[derive(Serialize)]
pub struct AdminMeData {
    pub id: i32,
    pub nickname: String,
    pub email: String,
    pub is_admin: bool,
}

/// GET /api/admin/me — 获取当前管理员信息
pub async fn get_me(
    State(state): State<AppState>,
    AdminUser(user_id): AdminUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let user = match db::find_user_by_id(pool, user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "用户不存在"),
        Err(e) => {
            tracing::error!("查询管理员信息失败: {}", e);
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
        AdminMeData {
            id: user.id,
            nickname: user.nickname,
            email: user.email,
            is_admin: user.is_admin,
        },
    )
}

#[derive(Deserialize)]
pub struct PageQuery {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

fn pagination(params: &PageQuery) -> (i64, i64) {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    (page, page_size)
}

#[derive(Serialize)]
pub struct ListData<T: Serialize> {
    pub list: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// GET /api/admin/users — 用户管理列表
pub async fn list_users(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Query(params): Query<PageQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let keyword = params
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let (page, page_size) = pagination(&params);

    match db::list_admin_users(pool, keyword, page, page_size).await {
        Ok((list, total)) => response::ok(
            StatusCode::OK,
            200,
            "success",
            ListData {
                list,
                total,
                page,
                page_size,
            },
        ),
        Err(e) => {
            tracing::error!("查询用户列表失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateUserStatusRequest {
    pub is_disabled: bool,
}

/// PUT /api/admin/users/{id}/status — 启用/禁用用户
pub async fn update_user_status(
    State(state): State<AppState>,
    AdminUser(current_admin): AdminUser,
    Path(user_id): Path<i32>,
    Json(payload): Json<UpdateUserStatusRequest>,
) -> axum::response::Response {
    if user_id == current_admin && payload.is_disabled {
        return response::error(StatusCode::BAD_REQUEST, 400, "不能禁用当前登录的管理员");
    }
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    match db::update_user_disabled(pool, user_id, payload.is_disabled).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            if payload.is_disabled {
                "用户已禁用"
            } else {
                "用户已启用"
            },
            serde_json::json!({ "user_id": user_id, "is_disabled": payload.is_disabled }),
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "用户不存在"),
        Err(e) => {
            tracing::error!("更新用户状态失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateUserAdminRequest {
    pub is_admin: bool,
}

/// PUT /api/admin/users/{id}/admin — 设置/取消管理员
pub async fn update_user_admin(
    State(state): State<AppState>,
    AdminUser(current_admin): AdminUser,
    Path(user_id): Path<i32>,
    Json(payload): Json<UpdateUserAdminRequest>,
) -> axum::response::Response {
    if user_id == current_admin && !payload.is_admin {
        return response::error(StatusCode::BAD_REQUEST, 400, "不能取消自己的管理员权限");
    }
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    match db::update_user_admin(pool, user_id, payload.is_admin).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            if payload.is_admin {
                "已设置为管理员"
            } else {
                "已取消管理员"
            },
            serde_json::json!({ "user_id": user_id, "is_admin": payload.is_admin }),
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "用户不存在"),
        Err(e) => {
            tracing::error!("更新管理员状态失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

/// PUT /api/admin/users/{id}/password — 重置用户密码
pub async fn reset_user_password(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(user_id): Path<i32>,
    Json(payload): Json<ResetPasswordRequest>,
) -> axum::response::Response {
    if payload.new_password.len() < 6 {
        return response::error(StatusCode::BAD_REQUEST, 400, "新密码至少6位");
    }
    let hash = match auth::hash_password(&payload.new_password) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!("密码加密失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    match db::reset_user_password(pool, user_id, &hash).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            "密码已重置",
            serde_json::json!({ "user_id": user_id }),
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "用户不存在"),
        Err(e) => {
            tracing::error!("重置用户密码失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

/// GET /api/admin/resources — 资源审核列表
pub async fn list_resources(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Query(params): Query<PageQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let status = params
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let keyword = params
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let (page, page_size) = pagination(&params);

    match db::list_admin_resources(pool, status, keyword, page, page_size).await {
        Ok((list, total)) => response::ok(
            StatusCode::OK,
            200,
            "success",
            ListData {
                list,
                total,
                page,
                page_size,
            },
        ),
        Err(e) => {
            tracing::error!("查询资源审核列表失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

#[derive(Deserialize)]
pub struct ReviewResourceRequest {
    pub status: String,
    pub reject_reason: Option<String>,
}

/// PUT /api/admin/resources/{id}/review — 资源审核
pub async fn review_resource(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(resource_id): Path<i32>,
    Json(payload): Json<ReviewResourceRequest>,
) -> axum::response::Response {
    let status = payload.status.trim();
    if status != "已上线" && status != "未通过" {
        return response::error(StatusCode::BAD_REQUEST, 400, "审核状态仅支持已上线/未通过");
    }
    let reject_reason = payload
        .reject_reason
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if status == "未通过" && reject_reason.is_none() {
        return response::error(StatusCode::BAD_REQUEST, 400, "审核未通过时必须填写原因");
    }
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    match db::review_resource(pool, resource_id, status, reject_reason).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            "资源审核完成",
            serde_json::json!({ "resource_id": resource_id, "status": status }),
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "资源不存在"),
        Err(e) => {
            tracing::error!("审核资源失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

#[derive(Deserialize)]
pub struct ToggleHotRequest {
    pub is_hot: bool,
}

/// PUT /api/admin/resources/{id}/hot — 设置/取消热门资源
pub async fn toggle_resource_hot(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(resource_id): Path<i32>,
    Json(payload): Json<ToggleHotRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    match db::toggle_resource_hot(pool, resource_id, payload.is_hot).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            "热门状态已更新",
            serde_json::json!({ "resource_id": resource_id, "is_hot": payload.is_hot }),
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "资源不存在"),
        Err(e) => {
            tracing::error!("更新热门状态失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

/// DELETE /api/admin/resources/{id} — 删除资源
pub async fn delete_resource(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(resource_id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    match db::delete_resource(pool, resource_id).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            "资源已删除",
            serde_json::json!({ "resource_id": resource_id }),
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "资源不存在"),
        Err(e) => {
            tracing::error!("删除资源失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

/// GET /api/admin/votes — 考点审核列表
pub async fn list_votes(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Query(params): Query<PageQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let status = params
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let keyword = params
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let (page, page_size) = pagination(&params);

    match db::list_admin_votes(pool, status, keyword, page, page_size).await {
        Ok((list, total)) => response::ok(
            StatusCode::OK,
            200,
            "success",
            ListData {
                list,
                total,
                page,
                page_size,
            },
        ),
        Err(e) => {
            tracing::error!("查询考点审核列表失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

#[derive(Deserialize)]
pub struct ReviewVoteRequest {
    pub status: String,
    pub reject_reason: Option<String>,
}

/// PUT /api/admin/votes/{id}/review — 考点审核
pub async fn review_vote(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(vote_id): Path<i32>,
    Json(payload): Json<ReviewVoteRequest>,
) -> axum::response::Response {
    let status = payload.status.trim();
    if status != "已通过" && status != "已拒绝" {
        return response::error(StatusCode::BAD_REQUEST, 400, "审核状态仅支持已通过/已拒绝");
    }
    let reject_reason = payload
        .reject_reason
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if status == "已拒绝" && reject_reason.is_none() {
        return response::error(StatusCode::BAD_REQUEST, 400, "审核拒绝时必须填写原因");
    }
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    match db::review_vote(pool, vote_id, status, reject_reason).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            "考点审核完成",
            serde_json::json!({ "vote_id": vote_id, "status": status }),
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "考点不存在"),
        Err(e) => {
            tracing::error!("审核考点失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

/// DELETE /api/admin/votes/{id} — 删除考点
pub async fn delete_vote(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(vote_id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    match db::delete_vote(pool, vote_id).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            "考点已删除",
            serde_json::json!({ "vote_id": vote_id }),
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "考点不存在"),
        Err(e) => {
            tracing::error!("删除考点失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

/// GET /api/admin/teams — 小队管理列表
pub async fn list_teams(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Query(params): Query<PageQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let keyword = params
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let (page, page_size) = pagination(&params);

    match db::list_admin_teams(pool, keyword, page, page_size).await {
        Ok((list, total)) => response::ok(
            StatusCode::OK,
            200,
            "success",
            ListData {
                list,
                total,
                page,
                page_size,
            },
        ),
        Err(e) => {
            tracing::error!("查询小队列表失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

/// DELETE /api/admin/teams/{id} — 解散小队
pub async fn delete_team(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(team_id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    match db::delete_team(pool, team_id).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            "小队已解散",
            serde_json::json!({ "team_id": team_id }),
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "小队不存在"),
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

#[derive(Serialize)]
pub struct DashboardUsersData {
    pub total: i64,
    pub today_new: i64,
    pub disabled: i64,
}

#[derive(Serialize)]
pub struct DashboardContentData {
    pub resources_total: i64,
    pub resources_pending: i64,
    pub votes_total: i64,
    pub votes_pending: i64,
    pub teams_total: i64,
    pub checkins_total: i64,
    pub documents_total: i64,
    pub notes_total: i64,
    pub exams_total: i64,
}

#[derive(Serialize)]
pub struct DashboardData {
    pub users: DashboardUsersData,
    pub content: DashboardContentData,
    pub checkin_trend: Vec<db::CheckinTrendItem>,
    pub resource_categories: Vec<db::ResourceCategoryCount>,
    pub recent_users: Vec<db::AdminUserListItem>,
    pub recent_resources: Vec<db::AdminResourceListItem>,
    pub recent_votes: Vec<db::AdminVoteListItem>,
}

/// GET /api/admin/stats/dashboard — 管理端统计看板
pub async fn dashboard(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let stats = match db::admin_dashboard_stats(pool).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("统计看板数据失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let checkin_trend = match db::checkin_trend(pool, 7).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("查询打卡趋势失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let resource_categories = match db::resource_categories(pool).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("查询资源分类统计失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let (recent_users, _) = match db::list_admin_users(pool, None, 1, 5).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("查询最近用户失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let (recent_resources, _) = match db::list_admin_resources(pool, None, None, 1, 5).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("查询最近资源失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let (recent_votes, _) = match db::list_admin_votes(pool, None, None, 1, 5).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("查询最近考点失败: {}", e);
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
        DashboardData {
            users: DashboardUsersData {
                total: stats.users_total,
                today_new: stats.users_today_new,
                disabled: stats.users_disabled,
            },
            content: DashboardContentData {
                resources_total: stats.resources_total,
                resources_pending: stats.resources_pending,
                votes_total: stats.votes_total,
                votes_pending: stats.votes_pending,
                teams_total: stats.teams_total,
                checkins_total: stats.checkins_total,
                documents_total: stats.documents_total,
                notes_total: stats.notes_total,
                exams_total: stats.exams_total,
            },
            checkin_trend,
            resource_categories,
            recent_users,
            recent_resources,
            recent_votes,
        },
    )
}