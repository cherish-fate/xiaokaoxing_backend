use std::collections::BTreeMap;

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub code: u16,
    pub message: String,
    pub data: T,
}

pub fn ok<T>(message: impl Into<String>, data: T) -> Json<ApiResponse<T>>
where
    T: Serialize,
{
    Json(ApiResponse {
        code: 0,
        message: message.into(),
        data,
    })
}

pub fn error(
    status: StatusCode,
    code: u16,
    message: impl Into<String>,
    data: Option<BTreeMap<String, String>>,
) -> impl IntoResponse {
    (
        status,
        Json(ApiResponse {
            code,
            message: message.into(),
            data,
        }),
    )
}

pub fn message<T>(
    status: StatusCode,
    code: u16,
    message: impl Into<String>,
    data: T,
) -> impl IntoResponse
where
    T: Serialize,
{
    (
        status,
        Json(ApiResponse {
            code,
            message: message.into(),
            data,
        }),
    )
}
