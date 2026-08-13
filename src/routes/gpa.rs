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
    state::AppState,
};

// ============ 绩点算法 ============

pub fn parse_f64(s: &str) -> f64 {
    s.trim().parse().unwrap_or(0.0)
}

pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

pub fn score_to_gpa(score: f64, algorithm: &str) -> f64 {
    let value = match algorithm {
        "pku" => {
            if score >= 90.0 {
                4.0
            } else if score >= 85.0 {
                3.7
            } else if score >= 80.0 {
                3.3
            } else if score >= 75.0 {
                3.0
            } else if score >= 70.0 {
                2.7
            } else if score >= 65.0 {
                2.3
            } else if score >= 60.0 {
                2.0
            } else {
                0.0
            }
        }
        "wes" => {
            if score >= 85.0 {
                4.0
            } else if score >= 80.0 {
                3.7
            } else if score >= 75.0 {
                3.3
            } else if score >= 70.0 {
                3.0
            } else if score >= 65.0 {
                2.7
            } else if score >= 60.0 {
                2.3
            } else {
                0.0
            }
        }
        _ => {
            if score >= 90.0 {
                4.0
            } else if score >= 86.0 {
                3.7
            } else if score >= 83.0 {
                3.3
            } else if score >= 80.0 {
                3.0
            } else if score >= 76.0 {
                2.7
            } else if score >= 73.0 {
                2.3
            } else if score >= 70.0 {
                2.0
            } else if score >= 66.0 {
                1.7
            } else if score >= 63.0 {
                1.3
            } else if score >= 60.0 {
                1.0
            } else {
                0.0
            }
        }
    };
    round2(value)
}

pub fn grade_to_gpa(grade: &str) -> f64 {
    let value = match grade.trim().to_ascii_uppercase().as_str() {
        "A+" | "A" => 4.0,
        "A-" => 3.7,
        "B+" => 3.3,
        "B" => 3.0,
        "B-" => 2.7,
        "C+" => 2.3,
        "C" => 2.0,
        "C-" => 1.7,
        "D+" => 1.3,
        "D" => 1.0,
        _ => 0.0,
    };
    round2(value)
}

pub fn course_gpa_value(course: &db::CourseGrade, algorithm: &str) -> f64 {
    if let Some(score) = course.score.as_deref() {
        return score_to_gpa(parse_f64(score), algorithm);
    }
    if let Some(grade) = course.grade.as_deref() {
        return grade_to_gpa(grade);
    }
    0.0
}

pub fn weighted_gpa(courses: &[db::CourseGrade], algorithm: &str) -> f64 {
    let mut total = 0.0;
    let mut credits = 0.0;
    for course in courses {
        let credit = parse_f64(&course.credit);
        total += course_gpa_value(course, algorithm) * credit;
        credits += credit;
    }
    if credits > 0.0 {
        round2(total / credits)
    } else {
        0.0
    }
}

pub fn total_credits(courses: &[db::CourseGrade]) -> f64 {
    round2(courses.iter().map(|c| parse_f64(&c.credit)).sum())
}

pub fn format_decimal(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}

// ============ 学期接口 ============

#[derive(Serialize)]
pub struct SemesterItem {
    pub id: i32,
    pub name: String,
    pub year: i32,
    pub course_count: i64,
}

#[derive(Serialize)]
pub struct SemesterListData {
    pub list: Vec<SemesterItem>,
}

#[derive(Deserialize)]
pub struct CreateSemesterRequest {
    pub name: String,
    pub year: i32,
}

#[derive(Serialize)]
pub struct SemesterCreatedData {
    pub id: i32,
    pub name: String,
    pub year: i32,
}

/// GET /api/gpa/semesters
pub async fn list_semesters(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let semesters = match db::find_semesters_with_counts(pool, user_id).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("查询学期列表失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let list: Vec<SemesterItem> = semesters
        .into_iter()
        .map(|s| SemesterItem {
            id: s.id,
            name: s.name,
            year: s.year,
            course_count: s.course_count,
        })
        .collect();
    response::ok(StatusCode::OK, 200, "success", SemesterListData { list })
}

/// POST /api/gpa/semesters
pub async fn create_semester(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreateSemesterRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let name = payload.name.trim();
    if name.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "学期名称不能为空");
    }
    if name.chars().count() > 20 {
        return response::error(StatusCode::BAD_REQUEST, 400, "学期名称最多20个字符");
    }
    if !(1990..=2100).contains(&payload.year) {
        return response::error(
            StatusCode::BAD_REQUEST,
            400,
            "学年格式无效，应为1990-2100之间的整数",
        );
    }
    match db::create_semester(pool, user_id, name, payload.year).await {
        Ok(semester) => response::ok(
            StatusCode::CREATED,
            201,
            "学期创建成功",
            SemesterCreatedData {
                id: semester.id,
                name: semester.name,
                year: semester.year,
            },
        ),
        Err(e) => {
            if db::is_duplicate_key_anyhow(&e) {
                return response::error(StatusCode::CONFLICT, 409, "该学期已存在");
            }
            tracing::error!("创建学期失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

/// DELETE /api/gpa/semesters/{id}
pub async fn delete_semester(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let semester = match db::find_semester_by_id(pool, id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "学期不存在");
        }
        Err(e) => {
            tracing::error!("查询学期失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    if semester.user_id != user_id {
        return response::error(StatusCode::NOT_FOUND, 404, "学期不存在");
    }
    match db::delete_semester_with_courses(pool, id).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            "学期删除成功",
            serde_json::Value::Null,
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "学期不存在"),
        Err(e) => {
            tracing::error!("删除学期失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 课程接口 ============

#[derive(Deserialize)]
pub struct CourseListQuery {
    pub semester_id: i32,
}

#[derive(Serialize)]
pub struct CourseItem {
    pub id: i32,
    pub name: String,
    pub credit: f64,
    pub score: Option<f64>,
    pub grade: Option<String>,
    pub r#type: String,
    pub gpa: f64,
}

#[derive(Serialize)]
pub struct CourseListData {
    pub semester: String,
    pub list: Vec<CourseItem>,
}

#[derive(Deserialize)]
pub struct SaveCourseRequest {
    pub id: Option<i32>,
    pub semester_id: i32,
    pub name: String,
    pub credit: f64,
    pub score: Option<f64>,
    pub grade: Option<String>,
    pub r#type: String,
}

#[derive(Serialize)]
pub struct CourseSavedData {
    pub id: i32,
    pub name: String,
    pub gpa: f64,
}

fn course_to_item(course: db::CourseGrade) -> CourseItem {
    let gpa = course_gpa_value(&course, "standard");
    CourseItem {
        id: course.id,
        name: course.name,
        credit: parse_f64(&course.credit),
        score: course.score.as_deref().map(parse_f64),
        grade: course.grade,
        r#type: course.r#type,
        gpa,
    }
}

/// GET /api/gpa/courses
pub async fn list_courses(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(query): Query<CourseListQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let semester = match db::find_semester_by_id(pool, query.semester_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "学期不存在");
        }
        Err(e) => {
            tracing::error!("查询学期失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    if semester.user_id != user_id {
        return response::error(StatusCode::NOT_FOUND, 404, "学期不存在");
    }
    let courses = match db::find_courses_by_semester(pool, query.semester_id).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("查询课程列表失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let list = courses.into_iter().map(course_to_item).collect();
    response::ok(
        StatusCode::OK,
        200,
        "success",
        CourseListData {
            semester: semester.name,
            list,
        },
    )
}

fn validate_course_payload(
    name: &str,
    credit: f64,
    score: Option<f64>,
    grade: Option<&str>,
    r#type: &str,
) -> Option<String> {
    if name.trim().is_empty() {
        return Some("课程名称不能为空".to_string());
    }
    if name.trim().chars().count() > 50 {
        return Some("课程名称最多50个字符".to_string());
    }
    if !(0.5..=10.0).contains(&credit) || ((credit * 2.0 - (credit * 2.0).round()).abs() > 1e-9) {
        return Some("学分应为0.5-10，且步长为0.5".to_string());
    }
    if let Some(s) = score {
        if !(0.0..=100.0).contains(&s) {
            return Some("百分制成绩应在0-100之间".to_string());
        }
    }
    let grade_ok = grade.map(|g| !g.trim().is_empty()).unwrap_or(false);
    if score.is_none() && !grade_ok {
        return Some("百分制成绩与等级制成绩需二选一".to_string());
    }
    if !matches!(r#type.trim(), "必修" | "选修" | "公共") {
        return Some("课程类型仅支持：必修/选修/公共".to_string());
    }
    None
}

/// POST /api/gpa/courses
pub async fn save_course(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<SaveCourseRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let name = payload.name.trim();
    let grade = payload.grade.as_deref().map(str::trim);
    let course_type = payload.r#type.trim();
    if let Some(msg) = validate_course_payload(name, payload.credit, payload.score, grade, course_type)
    {
        return response::error(StatusCode::BAD_REQUEST, 400, msg);
    }

    let score_str = payload.score.map(|s| format!("{:.1}", s));
    let grade_str = grade.map(|g| g.to_string());
    let credit_str = format!("{:.1}", payload.credit);
    let gpa = if let Some(s) = payload.score {
        score_to_gpa(s, "standard")
    } else {
        grade.map(grade_to_gpa).unwrap_or(0.0)
    };
    let gpa_str = format!("{:.2}", gpa);

    if let Some(id) = payload.id {
        let existing = match db::find_course_by_id(pool, id).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                return response::error(StatusCode::NOT_FOUND, 404, "课程不存在");
            }
            Err(e) => {
                tracing::error!("查询课程失败: {}", e);
                return response::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "服务器内部错误，请稍后重试",
                );
            }
        };
        let semester = match db::find_semester_by_id(pool, existing.semester_id).await {
            Ok(Some(s)) => s,
            _ => {
                return response::error(StatusCode::NOT_FOUND, 404, "课程不存在");
            }
        };
        if semester.user_id != user_id {
            return response::error(StatusCode::NOT_FOUND, 404, "课程不存在");
        }
        match db::update_course(
            pool,
            id,
            payload.semester_id,
            name,
            &credit_str,
            score_str.as_deref(),
            grade_str.as_deref(),
            course_type,
            Some(&gpa_str),
        )
        .await
        {
            Ok(Some(course)) => response::ok(
                StatusCode::OK,
                200,
                "课程保存成功",
                CourseSavedData {
                    id: course.id,
                    name: course.name,
                    gpa,
                },
            ),
            Ok(None) => response::error(StatusCode::NOT_FOUND, 404, "课程不存在"),
            Err(e) => {
                tracing::error!("更新课程失败: {}", e);
                response::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "服务器内部错误，请稍后重试",
                )
            }
        }
    } else {
        let semester = match db::find_semester_by_id(pool, payload.semester_id).await {
            Ok(Some(s)) => s,
            Ok(None) => {
                return response::error(StatusCode::NOT_FOUND, 404, "学期不存在");
            }
            Err(e) => {
                tracing::error!("查询学期失败: {}", e);
                return response::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "服务器内部错误，请稍后重试",
                );
            }
        };
        if semester.user_id != user_id {
            return response::error(StatusCode::NOT_FOUND, 404, "学期不存在");
        }
        match db::create_course(
            pool,
            payload.semester_id,
            name,
            &credit_str,
            score_str.as_deref(),
            grade_str.as_deref(),
            course_type,
            Some(&gpa_str),
        )
        .await
        {
            Ok(course) => response::ok(
                StatusCode::OK,
                200,
                "课程保存成功",
                CourseSavedData {
                    id: course.id,
                    name: course.name,
                    gpa,
                },
            ),
            Err(e) => {
                tracing::error!("创建课程失败: {}", e);
                response::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "服务器内部错误，请稍后重试",
                )
            }
        }
    }
}

/// DELETE /api/gpa/courses/{id}
pub async fn delete_course(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(id): Path<i32>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let course = match db::find_course_by_id(pool, id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return response::error(StatusCode::NOT_FOUND, 404, "课程不存在");
        }
        Err(e) => {
            tracing::error!("查询课程失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let semester = match db::find_semester_by_id(pool, course.semester_id).await {
        Ok(Some(s)) => s,
        _ => {
            return response::error(StatusCode::NOT_FOUND, 404, "课程不存在");
        }
    };
    if semester.user_id != user_id {
        return response::error(StatusCode::NOT_FOUND, 404, "课程不存在");
    }
    match db::delete_course(pool, id).await {
        Ok(true) => response::ok(
            StatusCode::OK,
            200,
            "课程删除成功",
            serde_json::Value::Null,
        ),
        Ok(false) => response::error(StatusCode::NOT_FOUND, 404, "课程不存在"),
        Err(e) => {
            tracing::error!("删除课程失败: {}", e);
            response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            )
        }
    }
}

// ============ 绩点计算 ============

#[derive(Deserialize)]
pub struct CalculateQuery {
    pub algorithm: Option<String>,
}

#[derive(Serialize)]
pub struct GpaTarget {
    pub value: f64,
    pub gap: f64,
    pub status: String,
}

#[derive(Serialize)]
pub struct GpaTargets {
    #[serde(rename = "保研线")]
    pub baoyan: GpaTarget,
    #[serde(rename = "考研线")]
    pub kaoyan: GpaTarget,
    #[serde(rename = "毕业线")]
    pub biye: GpaTarget,
}

#[derive(Serialize)]
pub struct GpaWarning {
    pub course_name: String,
    pub score: Option<f64>,
    pub credit: f64,
    pub message: String,
}

#[derive(Serialize)]
pub struct CalculateData {
    pub algorithm: String,
    pub current_gpa: f64,
    pub total_credits: f64,
    pub targets: GpaTargets,
    pub warnings: Vec<GpaWarning>,
    pub failed_courses: i64,
    pub failed_credits: f64,
}

/// GET /api/gpa/calculate
pub async fn calculate_gpa(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(query): Query<CalculateQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let algorithm = query.algorithm.unwrap_or_else(|| "standard".to_string());
    if !matches!(algorithm.as_str(), "standard" | "pku" | "wes" | "custom") {
        return response::error(
            StatusCode::BAD_REQUEST,
            400,
            "算法仅支持：standard/pku/wes/custom",
        );
    }
    // custom 算法暂未开放自定义阈值，先按国标4.0计算，接口结构保持不变
    let calc_algorithm = if algorithm == "custom" {
        "standard"
    } else {
        algorithm.as_str()
    };

    let semesters = match db::find_semesters_with_counts(pool, user_id).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("查询学期列表失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };

    let mut all_courses = Vec::new();
    for semester in semesters {
        match db::find_courses_by_semester(pool, semester.id).await {
            Ok(courses) => all_courses.extend(courses),
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

    let current_gpa = weighted_gpa(&all_courses, calc_algorithm);
    let credits = total_credits(&all_courses);

    let make_target = |value: f64| GpaTarget {
        value,
        gap: round2(current_gpa - value),
        status: if current_gpa >= value {
            "已超出".to_string()
        } else {
            "差".to_string()
        },
    };

    let mut failed_courses = 0;
    let mut failed_credits = 0.0;
    let mut warnings = Vec::new();
    for course in &all_courses {
        let score = course.score.as_deref().map(parse_f64);
        let is_failed = match score {
            Some(s) => s < 60.0,
            None => course
                .grade
                .as_deref()
                .map(|g| g.trim().eq_ignore_ascii_case("f"))
                .unwrap_or(false),
        };
        if !is_failed {
            continue;
        }
        let credit = parse_f64(&course.credit);
        failed_courses += 1;
        failed_credits += credit;
        let score_text = score
            .map(format_decimal)
            .unwrap_or_else(|| "F".to_string());
        warnings.push(GpaWarning {
            course_name: course.name.clone(),
            score,
            credit,
            message: format!(
                "{} 成绩 {} 分，预计挂科。将影响 {} 学分，建议补考或重修。",
                course.name,
                score_text,
                format_decimal(credit)
            ),
        });
    }

    response::ok(
        StatusCode::OK,
        200,
        "success",
        CalculateData {
            algorithm,
            current_gpa,
            total_credits: credits,
            targets: GpaTargets {
                baoyan: make_target(3.80),
                kaoyan: make_target(3.50),
                biye: make_target(2.00),
            },
            warnings,
            failed_courses,
            failed_credits: round2(failed_credits),
        },
    )
}
