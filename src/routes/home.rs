use axum::{
    extract::State,
    http::StatusCode,
};
use serde::Serialize;

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    state::AppState,
};

#[derive(Serialize)]
pub struct UserInfo {
    pub id: i32,
    pub nickname: String,
    pub avatar_url: Option<String>,
}

#[derive(Serialize)]
pub struct ExamCountdown {
    pub has_exam: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exam_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exam_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub countdown_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub countdown_text: Option<String>,
}

#[derive(Serialize)]
pub struct TodayProgress {
    pub completed: i64,
    pub total: i64,
    pub progress_percentage: i64,
}

#[derive(Serialize)]
pub struct RecentExamItem {
    pub id: i32,
    pub name: String,
    pub exam_date: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub location: Option<String>,
    pub days_remaining: i64,
    pub days_text: String,
}

#[derive(Serialize)]
pub struct TaskItem {
    pub id: i32,
    pub task_name: String,
    pub plan_date: String,
    pub is_completed: bool,
}

#[derive(Serialize)]
pub struct ResourceItem {
    pub id: i32,
    pub title: String,
    pub type_tag: String,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct HomeData {
    pub user: UserInfo,
    pub exam_countdown: ExamCountdown,
    pub today_progress: TodayProgress,
    pub recent_exams: Vec<RecentExamItem>,
    pub today_tasks: Vec<TaskItem>,
    pub recommended_resources: Vec<ResourceItem>,
}

pub async fn get_home(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "数据库未连接",
        );
    };

    // 查询用户信息
    let user = match db::find_user_by_id(pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return response::error(
                StatusCode::NOT_FOUND,
                404,
                "用户不存在",
            );
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

    let user_info = UserInfo {
        id: user.id,
        nickname: user.nickname.clone(),
        avatar_url: user.avatar_url.clone(),
    };

    // 查询最近考试（倒计时）
    let exam_countdown = match db::find_upcoming_exam(pool, user_id).await {
        Ok(Some(exam)) => {
            let today = chrono::Local::now().naive_local().date();
            let days = (exam.exam_date - today).num_days();
            let countdown_text = if days == 0 {
                "今天".to_string()
            } else if days > 0 {
                format!("{}天", days)
            } else {
                format!("已过期{}天", -days)
            };
            ExamCountdown {
                has_exam: true,
                exam_name: Some(exam.name.clone()),
                exam_date: Some(exam.exam_date.format("%Y-%m-%d").to_string()),
                start_time: Some(exam.start_time.format("%H:%M").to_string()),
                end_time: exam.end_time.map(|t| t.format("%H:%M").to_string()),
                countdown_days: Some(days),
                countdown_text: Some(countdown_text),
            }
        }
        Ok(None) => ExamCountdown {
            has_exam: false,
            exam_name: None,
            exam_date: None,
            start_time: None,
            end_time: None,
            countdown_days: None,
            countdown_text: None,
        },
        Err(e) => {
            tracing::error!("查询最近考试失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 今日进度统计
    let (total, completed) = match db::count_exams_by_user(pool, user_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("统计考试数量失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let progress_percentage = if total > 0 {
        (completed * 100) / total
    } else {
        0
    };
    let today_progress = TodayProgress {
        completed,
        total,
        progress_percentage,
    };

    // 最近考试列表（默认3场）
    let recent_exams = match db::find_exams_by_user(pool, user_id, None).await {
        Ok(exams) => {
            let today = chrono::Local::now().naive_local().date();
            exams.into_iter()
                .take(3)
                .map(|exam| {
                    let days = (exam.exam_date - today).num_days();
                    let days_text = if days == 0 {
                        "今天".to_string()
                    } else if days > 0 {
                        format!("{}天后", days)
                    } else {
                        format!("已过期{}天", -days)
                    };
                    RecentExamItem {
                        id: exam.id,
                        name: exam.name,
                        exam_date: exam.exam_date.format("%Y-%m-%d").to_string(),
                        start_time: exam.start_time.format("%H:%M").to_string(),
                        end_time: exam.end_time.map(|t| t.format("%H:%M").to_string()),
                        location: exam.location,
                        days_remaining: days,
                        days_text,
                    }
                })
                .collect()
        }
        Err(e) => {
            tracing::error!("查询考试列表失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 今日任务
    let today_tasks = match db::find_today_tasks(pool, user_id).await {
        Ok(tasks) => {
            tasks.into_iter()
                .map(|t| TaskItem {
                    id: t.id,
                    task_name: t.task_name,
                    plan_date: t.plan_date.format("%Y-%m-%d").to_string(),
                    is_completed: t.is_completed,
                })
                .collect()
        }
        Err(e) => {
            tracing::error!("查询今日任务失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 推荐资源
    let recommended_resources = match db::find_recommended_resources(
        pool,
        &user.school_name,
        user.major_id,
        4,
    )
    .await
    {
        Ok(resources) => {
            resources
                .into_iter()
                .map(|r| ResourceItem {
                    id: r.id,
                    title: r.title,
                    type_tag: r.type_tag,
                    description: r.description,
                })
                .collect()
        }
        Err(e) => {
            tracing::error!("查询推荐资源失败: {}", e);
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
        HomeData {
            user: user_info,
            exam_countdown,
            today_progress,
            recent_exams,
            today_tasks,
            recommended_resources,
        },
    )
}
