use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post, put, delete},
};

pub mod ai;
pub mod auth;
pub mod exams;
pub mod health;
pub mod home;
pub mod majors;
pub mod prep;
pub mod resources;
pub mod tasks;
pub mod users;

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
        .route("/api/resources", get(resources::list_resources))
        .route("/api/resources/search", get(resources::search_resources))
        // 收藏 / 取消收藏
        .route("/api/resources/{id}/favorite", post(resources::toggle_favorite))
        // AI 对话（流式）与文件上传
        .route("/api/ai/chat", post(ai::chat))
        .route(
            "/api/ai/upload",
            post(ai::upload).layer(DefaultBodyLimit::max(11 * 1024 * 1024)),
        )
}
