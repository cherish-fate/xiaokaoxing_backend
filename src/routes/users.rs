use axum::{Json, extract::State, http::StatusCode};

use serde::{Deserialize, Serialize};

use crate::{
    auth, auth_ext::AuthenticatedUser, db, response,
    routes::{
        checkin::{UserCheckinStats, compute_user_checkin_stats},
        gpa::weighted_gpa,
    },
    state::AppState,
    utils::is_valid_email,
};

/// 注册请求体
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub nickname: String,
    pub email: String,
    pub password: String,
    pub school_name: String,
    pub major_id: i32,
}

/// 注册成功响应中的用户数据
#[derive(Serialize)]
pub struct RegisterData {
    pub id: i32,
    pub nickname: String,
    pub email: String,
    pub school_name: String,
    pub major_id: i32,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

/// POST /api/users/register — 用户注册
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "数据库未连接",
        );
    };

    let nickname = payload.nickname.trim();
    let email = payload.email.trim();
    let password = payload.password; // 密码不做 trim，允许前后空格
    let school_name = payload.school_name.trim();
    let major_id = payload.major_id;

    // ---- 参数校验 ----
    if nickname.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "昵称不能为空");
    }
    if email.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "邮箱不能为空");
    }
    if !is_valid_email(email) {
        return response::error(StatusCode::BAD_REQUEST, 400, "邮箱格式不正确");
    }
    if password.len() < 6 {
        return response::error(StatusCode::BAD_REQUEST, 400, "密码至少6位");
    }
    if school_name.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "学校名称不能为空");
    }

    // ---- 校验 major_id 是否存在 ----
    match db::find_major_by_id(pool, major_id).await {
        Ok(None) => {
            return response::error(StatusCode::BAD_REQUEST, 400, "专业ID无效");
        }
        Ok(Some(_)) => {}
        Err(e) => {
            tracing::error!("查询专业失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    }

    // ---- 检查邮箱是否已注册 ----
    match db::find_user_by_email(pool, email).await {
        Ok(Some(_)) => {
            return response::error(
                StatusCode::CONFLICT,
                409,
                "该邮箱已注册，请直接登录",
            );
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("查询用户失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    }

    // ---- 加密密码 ----
    let password_hash = match auth::hash_password(&password) {
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

    // ---- 创建用户 ----
    match db::create_user(pool, nickname, email, &password_hash, school_name, major_id).await {
        Ok(user) => {
            let created_at = user.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            response::ok(
                StatusCode::CREATED,
                201,
                "注册成功",
                RegisterData {
                    id: user.id,
                    nickname: user.nickname,
                    email: user.email,
                    school_name: user.school_name,
                    major_id: user.major_id,
                    avatar_url: user.avatar_url,
                    created_at,
                },
            )
        }
        Err(e) => {
            tracing::error!("创建用户失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 我的页：个人资料 ============

#[derive(Serialize)]
pub struct UserMeData {
    pub id: i32,
    pub nickname: String,
    pub email: String,
    pub school_name: String,
    pub major_id: i32,
    pub major_name: String,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

fn user_me_data(user: db::User, major_name: String) -> UserMeData {
    UserMeData {
        id: user.id,
        nickname: user.nickname,
        email: user.email,
        school_name: user.school_name,
        major_id: user.major_id,
        major_name,
        avatar_url: user.avatar_url,
        created_at: user.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    }
}

/// GET /api/users/me — 获取当前用户个人信息
pub async fn get_me(
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
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };

    let major_name = match db::find_major_by_id(pool, user.major_id).await {
        Ok(Some(m)) => m.name,
        Ok(None) => String::new(),
        Err(e) => {
            tracing::error!("查询专业失败: {}", e);
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };

    response::ok(StatusCode::OK, 200, "success", user_me_data(user, major_name))
}

#[derive(Deserialize)]
pub struct UpdateMeRequest {
    pub nickname: Option<String>,
    pub school_name: Option<String>,
    pub major_id: Option<i32>,
    pub avatar_url: Option<String>,
    pub password: Option<String>,
}

/// PUT /api/users/me — 更新当前用户个人信息
pub async fn update_me(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<UpdateMeRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    let user = match db::find_user_by_id(pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "用户不存在"),
        Err(e) => {
            tracing::error!("查询用户失败: {}", e);
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };

    let nickname = payload
        .nickname
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&user.nickname)
        .to_string();
    if nickname.chars().count() > 50 {
        return response::error(StatusCode::BAD_REQUEST, 400, "昵称最大50字符");
    }

    let school_name = payload
        .school_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&user.school_name)
        .to_string();
    if school_name.chars().count() > 100 {
        return response::error(StatusCode::BAD_REQUEST, 400, "学校名称最大100字符");
    }

    let major_id = payload.major_id.unwrap_or(user.major_id);
    if let Some(major_id) = payload.major_id {
        match db::find_major_by_id(pool, major_id).await {
            Ok(Some(_)) => {}
            Ok(None) => return response::error(StatusCode::BAD_REQUEST, 400, "专业ID无效"),
            Err(e) => {
                tracing::error!("查询专业失败: {}", e);
                return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
            }
        }
    }

    let avatar_url = payload
        .avatar_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    // 修改密码（可选）
    if let Some(password) = &payload.password {
        if !password.is_empty() {
            if password.len() < 6 {
                return response::error(StatusCode::BAD_REQUEST, 400, "密码至少6位");
            }
            let hash = match auth::hash_password(password) {
                Ok(hash) => hash,
                Err(e) => {
                    tracing::error!("密码加密失败: {}", e);
                    return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
                }
            };
            if let Err(e) = db::reset_user_password(pool, user_id, &hash).await {
                tracing::error!("更新密码失败: {}", e);
                return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
            }
        }
    }

    let updated = match db::update_user_profile(
        pool,
        user_id,
        &nickname,
        &school_name,
        major_id,
        avatar_url.as_deref(),
    )
    .await
    {
        Ok(Some(u)) => u,
        Ok(None) => return response::error(StatusCode::NOT_FOUND, 404, "用户不存在"),
        Err(e) => {
            tracing::error!("更新用户资料失败: {}", e);
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };

    let major_name = match db::find_major_by_id(pool, updated.major_id).await {
        Ok(Some(m)) => m.name,
        _ => String::new(),
    };

    response::ok(
        StatusCode::OK,
        200,
        "个人资料已更新",
        user_me_data(updated, major_name),
    )
}

// ============ 我的页：学习数据概览 ============

#[derive(Serialize)]
pub struct LearningStatsData {
    pub notes_count: i64,
    pub bookmarks_count: i64,
    pub documents_count: i64,
    pub current_gpa: f64,
    pub daily_question_count: i64,
    pub exam_subjects_count: i64,
}

async fn collect_learning_stats(
    pool: &sqlx::MySqlPool,
    user_id: i32,
) -> anyhow::Result<LearningStatsData> {
    let notes_count = db::count_notes_by_user(pool, user_id).await?;
    let bookmarks_count = db::count_bookmarks_by_user(pool, user_id).await?;
    let documents_count = db::count_documents(pool, user_id, None).await?;
    let courses = db::find_courses_by_user(pool, user_id).await?;
    let current_gpa = weighted_gpa(&courses, "standard");
    let daily_question_count = db::count_question_records_by_user(pool, user_id).await?;
    let exam_subjects_count = db::count_distinct_exam_names_by_user(pool, user_id).await?;

    Ok(LearningStatsData {
        notes_count,
        bookmarks_count,
        documents_count,
        current_gpa,
        daily_question_count,
        exam_subjects_count,
    })
}

/// GET /api/users/me/learning-stats — 获取学习数据概览
pub async fn get_learning_stats(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    match collect_learning_stats(pool, user_id).await {
        Ok(data) => response::ok(StatusCode::OK, 200, "success", data),
        Err(e) => {
            tracing::error!("获取学习数据概览失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 我的页：聚合统计 ============

#[derive(Serialize)]
pub struct MyContentStatsData {
    pub uploads_count: i64,
    pub votes_count: i64,
    pub submissions_count: i64,
    pub teams_count: i64,
}

#[derive(Serialize)]
pub struct MyStatsData {
    pub checkin: UserCheckinStats,
    pub learning: LearningStatsData,
    pub my_content: MyContentStatsData,
}

/// GET /api/users/me/stats — 获取我的页聚合统计
pub async fn get_my_stats(
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
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };

    let checkin = match compute_user_checkin_stats(pool, user_id, &user.school_name).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("计算打卡统计失败: {}", e);
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };

    let learning = match collect_learning_stats(pool, user_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("获取学习数据概览失败: {}", e);
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };

    let uploads_count = match db::count_resources_by_user(pool, user_id).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("统计上传资源失败: {}", e);
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };
    let votes_count = match db::count_vote_records_by_user(pool, user_id).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("统计投票记录失败: {}", e);
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };
    let submissions_count = match db::count_vote_submissions_by_user(pool, user_id).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("统计投稿记录失败: {}", e);
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };
    let teams_count = match db::count_team_memberships_by_user(pool, user_id).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("统计小队数量失败: {}", e);
            return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "服务器内部错误，请稍后重试");
        }
    };

    response::ok(
        StatusCode::OK,
        200,
        "success",
        MyStatsData {
            checkin,
            learning,
            my_content: MyContentStatsData {
                uploads_count,
                votes_count,
                submissions_count,
                teams_count,
            },
        },
    )
}
