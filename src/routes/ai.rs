use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Json,
    body::Body,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    state::AppState,
};

#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub conversation_id: Option<String>,
    pub attachment_url: Option<String>,
}

#[derive(Serialize)]
pub struct UploadData {
    pub file_url: String,
    pub file_name: String,
    pub file_size: i64,
}

/// 校验上传文件扩展名，返回小写扩展名（不含点）
fn allowed_extension(filename: &str) -> Option<String> {
    let ext = filename.rsplit('.').next()?.to_lowercase();
    match ext.as_str() {
        "pdf" | "doc" | "docx" | "txt" => Some(ext),
        _ => None,
    }
}

/// 构造一条 SSE 错误事件响应
fn sse_error(message: &str) -> axum::response::Response {
    let payload = serde_json::json!({ "type": "error", "message": message });
    let body = format!("data: {}\n\n", payload);
    (
        StatusCode::OK,
        [
            ("Content-Type", "text/event-stream"),
            ("Cache-Control", "no-cache"),
            ("Connection", "keep-alive"),
        ],
        Body::from(body),
    )
        .into_response()
}

/// POST /api/ai/chat — AI 对话（流式骨架）
///
/// 当前为骨架实现：存储用户消息后返回 SSE 错误事件，AI 回复逻辑后续接入。
pub async fn chat(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<ChatRequest>,
) -> axum::response::Response {
    let message = payload.message.trim();
    if message.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "消息内容不能为空");
    }
    if message.chars().count() > 500 {
        return response::error(StatusCode::BAD_REQUEST, 400, "消息内容最多500字符");
    }

    let Some(pool) = state.db.as_ref() else {
        return sse_error("AI 服务暂时不可用，请稍后重试");
    };

    // 生成或复用会话 ID
    let conversation_id = match payload.conversation_id.as_deref() {
        Some(c) if !c.trim().is_empty() => c.trim().to_string(),
        _ => {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            format!("conv_{}_{}", user_id, ts)
        }
    };

    let attachment = payload
        .attachment_url
        .as_deref()
        .map(|a| a.trim())
        .filter(|a| !a.is_empty());

    // 存储用户消息
    if let Err(e) =
        db::create_ai_message(pool, user_id, &conversation_id, "user", message, attachment).await
    {
        tracing::error!("存储用户对话记录失败: {}", e);
        return sse_error("AI 服务暂时不可用，请稍后重试");
    }

    // AI 回复尚未接入
    sse_error("AI 服务暂未配置，请稍后重试")
}

/// POST /api/ai/upload — 上传文件作为 AI 对话上下文
pub async fn upload(
    State(state): State<AppState>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    mut multipart: Multipart,
) -> axum::response::Response {
    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name != "file" {
            continue;
        }

        let original = field.file_name().unwrap_or("file").to_string();
        let bytes = match field.bytes().await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("读取文件内容失败: {}", e);
                return response::error(StatusCode::BAD_REQUEST, 400, "文件读取失败");
            }
        };

        // 校验文件格式
        let ext = match allowed_extension(&original) {
            Some(e) => e,
            None => {
                return response::error(
                    StatusCode::BAD_REQUEST,
                    400,
                    "不支持的文件格式，仅支持PDF/DOC/DOCX/TXT",
                );
            }
        };

        // 校验文件大小（最大 10MB）
        const MAX_SIZE: usize = 10 * 1024 * 1024;
        if bytes.len() > MAX_SIZE {
            return response::error(StatusCode::BAD_REQUEST, 400, "文件大小超过10MB限制");
        }

        // 生成唯一文件名并保存
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let stored_name = format!("{}.{}", ts, ext);
        let dir = std::path::Path::new(&state.config.upload_dir);
        if let Err(e) = tokio::fs::create_dir_all(dir).await {
            tracing::error!("创建上传目录失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
        let file_path = dir.join(&stored_name);
        if let Err(e) = tokio::fs::write(&file_path, &bytes).await {
            tracing::error!("保存文件失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }

        let relative = format!("uploads/{}", stored_name);
        let file_url = state.config.public_url(&relative);
        return response::ok(
            StatusCode::OK,
            200,
            "上传成功",
            UploadData {
                file_url,
                file_name: original,
                file_size: bytes.len() as i64,
            },
        );
    }

    response::error(StatusCode::BAD_REQUEST, 400, "未找到上传文件")
}
