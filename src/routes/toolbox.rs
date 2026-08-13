use std::collections::HashMap;

use axum::{
    extract::State,
    http::StatusCode,
};
use serde::Serialize;

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    routes::gpa::{round2, weighted_gpa},
    state::AppState,
};

fn parse_tags(raw: &Option<String>) -> Vec<String> {
    raw.as_deref()
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
}

#[derive(Serialize)]
pub struct DailyQuestionHome {
    pub has_answered: bool,
    pub question: Option<String>,
    pub subject: Option<String>,
    pub date: String,
}

#[derive(Serialize)]
pub struct NoteTagCount {
    pub name: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct NotesHome {
    pub count: i64,
    pub tags: Vec<NoteTagCount>,
}

#[derive(Serialize)]
pub struct GpaHome {
    pub current: f64,
    pub trend: f64,
    pub semester: Option<String>,
}

#[derive(Serialize)]
pub struct DocumentsHome {
    pub total: i64,
    pub categories: HashMap<String, i64>,
}

#[derive(Serialize)]
pub struct BookmarksHome {
    pub total: i64,
}

#[derive(Serialize)]
pub struct ToolboxHomeData {
    pub daily_question: DailyQuestionHome,
    pub notes: NotesHome,
    pub gpa: GpaHome,
    pub documents: DocumentsHome,
    pub bookmarks: BookmarksHome,
}

/// GET /api/toolbox/home
pub async fn get_toolbox_home(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };

    // 每日一问
    let today = chrono::Local::now().naive_local().date();
    let daily_question = match db::find_daily_question_by_date(pool, today).await {
        Ok(Some(q)) => {
            let record = match db::find_question_record(pool, user_id, q.id).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("查询答题记录失败: {}", e);
                    return response::error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        500,
                        "服务器内部错误，请稍后重试",
                    );
                }
            };
            DailyQuestionHome {
                has_answered: record.is_some(),
                question: Some(q.question),
                subject: Some(q.subject),
                date: q.date.format("%Y-%m-%d").to_string(),
            }
        }
        Ok(None) => DailyQuestionHome {
            has_answered: false,
            question: None,
            subject: None,
            date: today.format("%Y-%m-%d").to_string(),
        },
        Err(e) => {
            tracing::error!("查询今日每日一题失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 笔记数量与标签聚合
    let notes = match db::count_notes_by_user(pool, user_id).await {
        Ok(count) => {
            let tag_rows = match db::find_note_tags_by_user(pool, user_id).await {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!("查询笔记标签失败: {}", e);
                    return response::error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        500,
                        "服务器内部错误，请稍后重试",
                    );
                }
            };
            let mut counts: HashMap<String, i64> = HashMap::new();
            for raw in tag_rows {
                for tag in parse_tags(&Some(raw)) {
                    *counts.entry(tag).or_insert(0) += 1;
                }
            }
            let mut tags: Vec<NoteTagCount> = counts
                .into_iter()
                .map(|(name, count)| NoteTagCount { name, count })
                .collect();
            tags.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
            NotesHome { count, tags }
        }
        Err(e) => {
            tracing::error!("统计笔记数量失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 绩点：当前学期与上一学期差值
    let gpa = match db::find_semesters_with_counts(pool, user_id).await {
        Ok(semesters) => {
            let latest = semesters.first();
            let mut current = 0.0;
            let mut trend = 0.0;
            let mut semester_name = None;
            if let Some(latest_semester) = latest {
                semester_name = Some(latest_semester.name.clone());
                match db::find_courses_by_semester(pool, latest_semester.id).await {
                    Ok(courses) => {
                        current = weighted_gpa(&courses, "standard");
                    }
                    Err(e) => {
                        tracing::error!("查询课程列表失败: {}", e);
                        return response::error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            500,
                            "服务器内部错误，请稍后重试",
                        );
                    }
                }
                if let Some(prev) = semesters.get(1) {
                    match db::find_courses_by_semester(pool, prev.id).await {
                        Ok(prev_courses) => {
                            let prev_gpa = weighted_gpa(&prev_courses, "standard");
                            trend = round2(current - prev_gpa);
                        }
                        Err(e) => {
                            tracing::error!("查询课程列表失败: {}", e);
                            return response::error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                500,
                                "服务器内部错误，请稍后重试",
                            );
                        }
                    }
                }
            }
            GpaHome {
                current,
                trend,
                semester: semester_name,
            }
        }
        Err(e) => {
            tracing::error!("查询学期列表失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 文档数量与分类
    let documents = match db::count_documents(pool, user_id, None).await {
        Ok(total) => {
            let mut categories: HashMap<String, i64> = HashMap::new();
            categories.insert("真题".to_string(), 0);
            categories.insert("笔记".to_string(), 0);
            categories.insert("收藏".to_string(), 0);
            categories.insert("导出".to_string(), 0);
            match db::find_document_category_counts(pool, user_id).await {
                Ok(rows) => {
                    for row in rows {
                        if let Some(category) = row.category {
                            categories.insert(category, row.count);
                        }
                    }
                    DocumentsHome { total, categories }
                }
                Err(e) => {
                    tracing::error!("统计文档分类失败: {}", e);
                    return response::error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        500,
                        "服务器内部错误，请稍后重试",
                    );
                }
            }
        }
        Err(e) => {
            tracing::error!("统计文档数量失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    // 书签数量
    let bookmarks = match db::count_bookmarks_by_user(pool, user_id).await {
        Ok(total) => BookmarksHome { total },
        Err(e) => {
            tracing::error!("统计书签数量失败: {}", e);
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
        ToolboxHomeData {
            daily_question,
            notes,
            gpa,
            documents,
            bookmarks,
        },
    )
}
