use std::time::{SystemTime, UNIX_EPOCH};

use async_stream::stream;
use axum::{
    Json,
    body::Body,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    state::AppState,
};

/// 校考星 AI 备考助手系统提示词
const SYSTEM_PROMPT: &str = "你是校考星的AI备考助手，专门帮助大学生备考期末考试。你的职责包括：根据用户需求生成模拟试卷、拆解和分析复习资料、诊断薄弱知识点、提供备考建议和学习计划，答疑解难等。回答要简洁、重点突出";

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

/// 流式事件项：序列化为 `data: {json}\n\n` 文本，供 Body::from_stream 使用
type SseItem = Result<String, std::io::Error>;

fn sse_data(payload: &Value) -> SseItem {
    Ok(format!("data: {}\n\n", payload))
}

/// 构造一个 SSE 流式响应（text/event-stream）
fn sse_stream<S>(stream: S) -> axum::response::Response
where
    S: futures::Stream<Item = SseItem> + Send + 'static,
{
    (
        StatusCode::OK,
        [
            ("Content-Type", "text/event-stream"),
            ("Cache-Control", "no-cache"),
            ("Connection", "keep-alive"),
        ],
        Body::from_stream(stream),
    )
        .into_response()
}

/// POST /api/ai/chat — AI 对话（流式）
///
/// 存储用户消息后，调用阿里云百炼（千问）兼容模式流式接口，
/// 将增量内容以 SSE 事件逐块转发给前端。
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

    // 校验 AI 配置
    let Some(api_key) = state.config.ai_api_key.clone() else {
        return sse_error("AI 服务暂未配置，请稍后重试");
    };
    let base_url = state.config.ai_base_url.trim_end_matches('/').to_string();
    let model = state.config.ai_model.clone();

    // 构造千问兼容模式请求体
    let req_body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": message }
        ],
        "stream": true
    });

    let client = reqwest::Client::new();
    let upstream = client
        .post(format!("{}/chat/completions", base_url))
        .bearer_auth(&api_key)
        .json(&req_body)
        .send()
        .await;

    let resp = match upstream {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("调用千问 API 失败: {:#}", e);
            return sse_error("AI 服务暂时不可用，请稍后重试");
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::error!("千问 API 返回非 2xx: status={}, body={}", status, body);
        return sse_error("AI 服务暂时不可用，请稍后重试");
    }

    // 用于流式处理中存储 AI 回复
    let pool_clone = pool.clone();
    let conv_id_clone = conversation_id.clone();

    let s = stream! {
        let mut byte_stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();
        let mut full = String::new();

        while let Some(chunk_res) = byte_stream.next().await {
            let bytes = match chunk_res {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("读取千问流式响应失败: {:#}", e);
                    yield sse_data(&serde_json::json!({ "type": "error", "message": "AI 服务响应中断，请稍后重试" }));
                    return;
                }
            };

            buf.extend_from_slice(&bytes);

            // 按行解析 SSE，行边界 b'\n' 不会出现在 UTF-8 多字节序列内部
            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line_with_nl: Vec<u8> = buf.drain(..=pos).collect();
                let line_bytes = &line_with_nl[..line_with_nl.len().saturating_sub(1)];
                let line = match std::str::from_utf8(line_bytes) {
                    Ok(s) => s.trim(),
                    Err(_) => continue,
                };
                if line.is_empty() {
                    continue;
                }
                let payload = match line.strip_prefix("data:") {
                    Some(p) => p.trim(),
                    None => continue,
                };

                if payload == "[DONE]" {
                    if let Err(e) = db::create_ai_message(
                        &pool_clone, user_id, &conv_id_clone, "assistant", &full, None
                    ).await {
                        tracing::warn!("存储 AI 回复记录失败: {:#}", e);
                    }
                    yield sse_data(&serde_json::json!({
                        "type": "done",
                        "conversation_id": conv_id_clone,
                        "full_content": full
                    }));
                    return;
                }

                if let Ok(v) = serde_json::from_str::<Value>(payload) {
                    let content = v
                        .get("choices")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("delta"))
                        .and_then(|d| d.get("content"))
                        .and_then(|c| c.as_str());
                    if let Some(content) = content {
                        if !content.is_empty() {
                            full.push_str(content);
                            yield sse_data(&serde_json::json!({
                                "type": "chunk",
                                "content": content
                            }));
                        }
                    }
                }
            }
        }

        // 流提前结束且未收到 [DONE]
        if full.is_empty() {
            yield sse_data(&serde_json::json!({ "type": "error", "message": "AI 服务无响应，请稍后重试" }));
        } else {
            if let Err(e) = db::create_ai_message(
                &pool_clone, user_id, &conv_id_clone, "assistant", &full, None
            ).await {
                tracing::warn!("存储 AI 回复记录失败: {:#}", e);
            }
            yield sse_data(&serde_json::json!({
                "type": "done",
                "conversation_id": conv_id_clone,
                "full_content": full
            }));
        }
    };

    sse_stream(s)
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
