use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post, put, delete},
};

pub mod ai;
pub mod auth;
pub mod checkin;
pub mod community;
pub mod exams;
pub mod health;
pub mod home;
pub mod majors;
pub mod prep;
pub mod resources;
pub mod tasks;
pub mod teams;
pub mod users;
pub mod votes;

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        // 健康检查
        .route("/", get(health::index))
        .route("/health", get(health::health))
        // 专业列表（注册页面下拉框数据源）
        .route("/api/majors", get(majors::list_majors))
        // 用户注册
        .route("/api/users/register", post(users::register))
        // 用户登录
        .route("/api/users/login", post(auth::login))
        // 首页聚合接口
        .route("/api/home", get(home::get_home))
        // 考试 CRUD
        .route("/api/exams", get(exams::list_exams))
        .route("/api/exams", post(exams::create_exam))
        .route("/api/exams/{id}", put(exams::update_exam))
        .route("/api/exams/{id}", delete(exams::delete_exam))
        // 任务接口
        .route("/api/tasks/today", get(tasks::get_today_tasks))
        .route("/api/tasks", post(tasks::create_task))
        .route("/api/tasks/{id}", put(tasks::update_task))
        .route("/api/tasks/{id}", delete(tasks::delete_task))
        // 推荐资源
        .route("/api/resources/recommended", get(resources::get_recommended_resources))
        // 备考中心
        .route("/api/prep/home", get(prep::get_prep_home))
        // 资源列表（分类筛选）与搜索
        .route("/api/resources", get(resources::list_resources).post(resources::upload_resource).layer(DefaultBodyLimit::max(50 * 1024 * 1024)))
        .route("/api/resources/search", get(resources::search_resources))
        .route("/api/resources/my-uploads", get(resources::get_my_uploads))
        // 收藏 / 取消收藏
        .route("/api/resources/{id}/favorite", post(resources::toggle_favorite))
        // AI 对话（流式）与文件上传
        .route("/api/ai/chat", post(ai::chat))
        .route(
            "/api/ai/upload",
            post(ai::upload).layer(DefaultBodyLimit::max(11 * 1024 * 1024)),
        )
        // ============ 社区模块 ============
        // 社区主页聚合
        .route("/api/community/home", get(community::get_home))
        // 打卡模块
        .route("/api/checkin/today", get(checkin::get_today))
        .route("/api/checkin", post(checkin::create_checkin))
        .route("/api/checkin/calendar", get(checkin::get_calendar))
        .route("/api/checkin/ranking", get(checkin::get_ranking))
        // 备考小队模块
        .route("/api/teams", get(teams::list_teams).post(teams::create_team))
        .route("/api/teams/{id}", get(teams::get_team_detail).delete(teams::dissolve_team))
        .route("/api/teams/{id}/join", post(teams::join_team))
        .route("/api/teams/{id}/applications", get(teams::list_applications))
        .route("/api/teams/{id}/applications/{application_id}", put(teams::process_application))
        .route("/api/teams/{id}/members", delete(teams::leave_team))
        // 考点投票模块
        .route("/api/votes", get(votes::list_votes).post(votes::create_vote))
        .route("/api/votes/my-votes", get(votes::get_my_votes))
        .route("/api/votes/my-submissions", get(votes::get_my_submissions))
        .route("/api/votes/{id}/vote", post(votes::cast_vote))
}
