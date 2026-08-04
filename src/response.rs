use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::Value;

/// 统一响应结构
///
/// ```json
/// { "code": 200, "message": "success", "data": { ... } }
/// ```
#[derive(Serialize)]
pub struct ApiResponse {
    pub code: u16,
    pub message: String,
    pub data: Value,
}

/// 成功响应（带数据）
pub fn ok<T: Serialize>(
    status: StatusCode,
    code: u16,
    message: impl Into<String>,
    data: T,
) -> Response {
    (
        status,
        Json(ApiResponse {
            code,
            message: message.into(),
            data: serde_json::to_value(data).unwrap_or(Value::Null),
        }),
    )
        .into_response()
}

/// 失败响应（data 为 null）
pub fn error(
    status: StatusCode,
    code: u16,
    message: impl Into<String>,
) -> Response {
    (
        status,
        Json(ApiResponse {
            code,
            message: message.into(),
            data: Value::Null,
        }),
    )
        .into_response()
}
