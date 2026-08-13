use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    auth_ext::AuthenticatedUser,
    db,
    response,
    state::AppState,
};

struct ExportItem {
    kind: &'static str,
    name: String,
    content: String,
}

#[derive(Deserialize)]
pub struct ExportRequest {
    pub file_ids: Vec<i32>,
    pub format: String,
    pub template: Option<String>,
}

#[derive(Serialize)]
pub struct ExportCreatedData {
    pub record_id: i32,
    pub file_url: String,
    pub file_size: i64,
    pub pages: i64,
    pub format: String,
    pub template: String,
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct HistoryItem {
    pub id: i32,
    pub file_count: i64,
    pub format: String,
    pub template: String,
    pub file_url: String,
    pub file_size: i64,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct HistoryData {
    pub total: i64,
    pub list: Vec<HistoryItem>,
}

#[derive(Serialize)]
pub struct PreviewData {
    pub preview_url: String,
    pub page_count: i64,
    pub estimated_size: i64,
}

fn parse_tags(raw: &Option<String>) -> Vec<String> {
    raw.as_deref()
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
}

async fn load_export_items(
    pool: &sqlx::MySqlPool,
    user_id: i32,
    ids: &[i32],
) -> Result<Vec<ExportItem>, Response> {
    let mut items = Vec::new();
    for id in ids {
        let doc = match db::find_document_by_id(pool, *id).await {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("查询导出文档失败: {}", e);
                return Err(response::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "服务器内部错误，请稍后重试",
                ));
            }
        };
        if let Some(doc) = doc {
            if doc.user_id != user_id {
                return Err(response::error(StatusCode::NOT_FOUND, 404, "部分文件不存在"));
            }
            items.push(ExportItem {
                kind: "document",
                name: doc.name,
                content: format!("{}\n{}", doc.file_url, doc.file_type),
            });
            continue;
        }

        let note = match db::find_note_by_id(pool, *id).await {
            Ok(n) => n,
            Err(e) => {
                tracing::error!("查询导出笔记失败: {}", e);
                return Err(response::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    500,
                    "服务器内部错误，请稍后重试",
                ));
            }
        };
        if let Some(note) = note {
            if note.user_id != user_id {
                return Err(response::error(StatusCode::NOT_FOUND, 404, "部分文件不存在"));
            }
            let tags = parse_tags(&note.tags);
            let tag_text = if tags.is_empty() {
                String::new()
            } else {
                format!("\n标签：{}", tags.join("/"))
            };
            items.push(ExportItem {
                kind: "note",
                name: note.title,
                content: format!("{}{}", note.content.unwrap_or_default(), tag_text),
            });
            continue;
        }

        return Err(response::error(StatusCode::NOT_FOUND, 404, "部分文件不存在"));
    }
    Ok(items)
}

fn validate_export_params(
    format: &str,
    template: &str,
) -> Result<(), Response> {
    if !matches!(format, "pdf" | "docx" | "image") {
        return Err(response::error(
            StatusCode::BAD_REQUEST,
            400,
            "导出格式仅支持：pdf/docx/image",
        ));
    }
    if !matches!(template, "minimal" | "academic" | "handwriting") {
        return Err(response::error(
            StatusCode::BAD_REQUEST,
            400,
            "模板仅支持：minimal/academic/handwriting",
        ));
    }
    Ok(())
}

fn estimated_pages(items: &[ExportItem], format: &str) -> usize {
    if format == "image" {
        return items.len().max(1);
    }
    let mut pages = 0;
    for item in items {
        pages += ((item.name.len() + item.content.len()) / 1200 + 1).max(1);
    }
    pages.max(1)
}

// ============ 导出接口 ============

/// POST /api/export
pub async fn create_export(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<ExportRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    if payload.file_ids.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "至少选择1个文件");
    }
    if payload.file_ids.len() > 100 {
        return response::error(StatusCode::BAD_REQUEST, 400, "单次最多导出100个文件");
    }
    let format = payload.format.trim().to_lowercase();
    let template = payload
        .template
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("minimal")
        .to_string();
    if let Err(res) = validate_export_params(&format, &template) {
        return res;
    }

    let items = match load_export_items(pool, user_id, &payload.file_ids).await {
        Ok(items) => items,
        Err(res) => return res,
    };
    if items.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "至少选择1个文件");
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let ext = if format == "image" { "png" } else { format.as_str() };
    let filename = format!("export_{}.{}", ts, ext);
    let timestamp = chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let bytes = match format.as_str() {
        "pdf" => build_pdf(&items, &template, &timestamp),
        "docx" => build_docx(&items, &template),
        _ => build_png(&items),
    };

    let dir = Path::new(&state.config.upload_dir).join("exports");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::error!("创建导出目录失败: {}", e);
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "服务器内部错误，请稍后重试",
        );
    }
    if let Err(e) = std::fs::write(dir.join(&filename), &bytes) {
        tracing::error!("写入导出文件失败: {}", e);
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "服务器内部错误，请稍后重试",
        );
    }

    let file_size = bytes.len() as i64;
    let file_url = state
        .config
        .public_url(&format!("uploads/exports/{}", filename));
    let pages = estimated_pages(&items, &format);
    let file_ids_json = serde_json::to_string(&payload.file_ids).unwrap_or_else(|_| "[]".to_string());

    let record = match db::create_export_record(
        pool,
        user_id,
        &file_ids_json,
        &format,
        &template,
        &file_url,
        file_size,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("创建导出记录失败: {}", e);
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
        "导出成功",
        ExportCreatedData {
            record_id: record.id,
            file_url,
            file_size,
            pages: pages as i64,
            format,
            template,
        },
    )
}

/// GET /api/export/history
pub async fn export_history(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(query): Query<HistoryQuery>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let total = match db::count_export_records(pool, user_id).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("统计导出记录失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let records = match db::find_export_records(pool, user_id, limit, offset).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("查询导出记录失败: {}", e);
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "服务器内部错误，请稍后重试",
            );
        }
    };
    let list: Vec<HistoryItem> = records
        .into_iter()
        .map(|r| {
            let file_count = r
                .file_ids
                .as_deref()
                .and_then(|s| serde_json::from_str::<Vec<i32>>(s).ok())
                .map(|ids| ids.len() as i64)
                .unwrap_or(0);
            HistoryItem {
                id: r.id,
                file_count,
                format: r.format,
                template: r.template,
                file_url: r.file_url,
                file_size: r.file_size,
                created_at: r.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            }
        })
        .collect();
    response::ok(
        StatusCode::OK,
        200,
        "success",
        HistoryData { total, list },
    )
}

/// POST /api/export/preview
pub async fn preview_export(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<ExportRequest>,
) -> axum::response::Response {
    let Some(pool) = state.db.as_ref() else {
        return response::error(StatusCode::INTERNAL_SERVER_ERROR, 500, "数据库未连接");
    };
    if payload.file_ids.is_empty() {
        return response::error(StatusCode::BAD_REQUEST, 400, "至少选择1个文件");
    }
    let format = payload.format.trim().to_lowercase();
    let template = payload
        .template
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("minimal")
        .to_string();
    if let Err(res) = validate_export_params(&format, &template) {
        return res;
    }
    let items = match load_export_items(pool, user_id, &payload.file_ids).await {
        Ok(items) => items,
        Err(res) => return res,
    };

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let filename = format!("preview_{}.png", ts);
    let bytes = build_png(&items);
    let dir = Path::new(&state.config.upload_dir).join("previews");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::error!("创建预览目录失败: {}", e);
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "服务器内部错误，请稍后重试",
        );
    }
    if let Err(e) = std::fs::write(dir.join(&filename), &bytes) {
        tracing::error!("写入预览文件失败: {}", e);
        return response::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            500,
            "服务器内部错误，请稍后重试",
        );
    }
    let preview_url = state
        .config
        .public_url(&format!("uploads/previews/{}", filename));
    response::ok(
        StatusCode::OK,
        200,
        "success",
        PreviewData {
            preview_url,
            page_count: estimated_pages(&items, &format) as i64,
            estimated_size: bytes.len() as i64,
        },
    )
}

// ============ 文件生成（无第三方依赖的最小实现） ============

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for n in 0..256 {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
        }
        table[n] = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

fn zip_store(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let name_bytes = name.as_bytes();
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(data);

        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name_bytes);
    }
    let central_offset = out.len() as u32;
    let central_size = central.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&central_size.to_le_bytes());
    out.extend_from_slice(&central_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn build_docx(items: &[ExportItem], template: &str) -> Vec<u8> {
    let mut body = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    );
    body.push_str(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#,
    );
    body.push_str(&format!(
        r#"<w:p><w:pPr><w:jc w:val="center"/></w:pPr><w:r><w:rPr><w:b/><w:sz w:val="36"/></w:rPr><w:t>{}</w:t></w:r></w:p>"#,
        escape_xml(&format!("校考星资料导出（{}）", template))
    ));
    for item in items {
        body.push_str(&format!(
            r#"<w:p><w:pPr><w:rPr><w:b/><w:sz w:val="28"/></w:rPr></w:pPr><w:r><w:rPr><w:b/><w:sz w:val="28"/></w:rPr><w:t>{}</w:t></w:r></w:p>"#,
            escape_xml(&format!("{} {}", item.kind, item.name))
        ));
        for line in item.content.lines() {
            body.push_str(&format!(
                r#"<w:p><w:r><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(line)
            ));
        }
    }
    body.push_str("</w:body></w:document>");

    let content_types = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#.to_vec();
    let rels = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#.to_vec();
    zip_store(&[
        ("[Content_Types].xml".to_string(), content_types),
        ("_rels/.rels".to_string(), rels),
        ("word/document.xml".to_string(), body.into_bytes()),
    ])
}

fn push_pdf_obj(out: &mut Vec<u8>, offsets: &mut Vec<usize>, num: usize, body: String) {
    offsets.push(out.len());
    out.extend_from_slice(format!("{} 0 obj\n", num).as_bytes());
    out.extend_from_slice(body.as_bytes());
    out.extend_from_slice(b"\nendobj\n");
}

fn ascii_pdf_line(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            out.push('?');
        }
    }
    out
}

fn escape_pdf_text(s: &str) -> String {
    s.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
}

fn build_pdf(items: &[ExportItem], template: &str, timestamp: &str) -> Vec<u8> {
    let mut lines = vec![
        "Xiaokaoxing Export".to_string(),
        format!("Template: {}", template),
        format!("Generated: {}", timestamp),
    ];
    for item in items {
        lines.push(String::new());
        lines.push(format!("[{}] {}", item.kind, item.name));
        for line in item.content.lines().take(30) {
            lines.push(ascii_pdf_line(line));
        }
    }
    let per_page = 50;
    let page_count = ((lines.len() + per_page - 1) / per_page).max(1);
    let first_page = 3;
    let font_obj = first_page + page_count;
    let content_objs: Vec<usize> = (0..page_count).map(|i| font_obj + 1 + i).collect();
    let total_objs = content_objs[page_count - 1];

    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");
    let mut offsets = Vec::new();
    push_pdf_obj(
        &mut out,
        &mut offsets,
        1,
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
    );
    let kids: Vec<String> = (first_page..first_page + page_count)
        .map(|n| format!("{} 0 R", n))
        .collect();
    push_pdf_obj(
        &mut out,
        &mut offsets,
        2,
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            page_count
        ),
    );
    for i in 0..page_count {
        push_pdf_obj(
            &mut out,
            &mut offsets,
            first_page + i,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 {} 0 R >> >> /Contents {} 0 R >>",
                font_obj, content_objs[i]
            ),
        );
    }
    push_pdf_obj(
        &mut out,
        &mut offsets,
        font_obj,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    );
    for i in 0..page_count {
        let mut stream = String::new();
        let mut y = 770;
        for line in lines.iter().skip(i * per_page).take(per_page) {
            stream.push_str(&format!(
                "BT /F1 11 Tf 64 {} Td ({}) Tj ET\n",
                y,
                escape_pdf_text(line)
            ));
            y -= 18;
        }
        push_pdf_obj(
            &mut out,
            &mut offsets,
            content_objs[i],
            format!(
                "<< /Length {} >>\nstream\n{}endstream",
                stream.len(),
                stream
            ),
        );
    }

    let xref_offset = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", total_objs + 1).as_bytes(),
    );
    for off in offsets {
        out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            total_objs + 1,
            xref_offset
        )
        .as_bytes(),
    );
    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut pos = 0;
    loop {
        let remaining = data.len() - pos;
        let len = remaining.min(65535);
        let final_byte = if pos + len == data.len() { 1 } else { 0 };
        out.push(final_byte);
        out.extend_from_slice(&(len as u16).to_le_bytes());
        out.extend_from_slice(&(!(len as u16)).to_le_bytes());
        out.extend_from_slice(&data[pos..pos + len]);
        pos += len;
        if pos >= data.len() {
            break;
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn build_png(items: &[ExportItem]) -> Vec<u8> {
    const W: usize = 640;
    const H: usize = 840;
    let mut pixels = vec![255u8; W * H * 3];
    let mut set = |x: usize, y: usize, rgb: (u8, u8, u8)| {
        if x < W && y < H {
            let idx = (y * W + x) * 3;
            pixels[idx] = rgb.0;
            pixels[idx + 1] = rgb.1;
            pixels[idx + 2] = rgb.2;
        }
    };

    for y in 0..48 {
        for x in 0..W {
            set(x, y, (24, 33, 72));
        }
    }
    let colors = [
        (216, 82, 73),
        (236, 175, 64),
        (52, 142, 108),
        (64, 122, 184),
        (132, 90, 184),
    ];
    for (i, color) in colors.iter().enumerate().take(items.len().min(5)) {
        for y in 72..82 {
            for x in (40 + i * 110)..(40 + i * 110 + 90) {
                set(x, y, *color);
            }
        }
    }
    for y in (100..H - 120).step_by(44) {
        for x in 40..W - 40 {
            set(x, y, (224, 224, 224));
        }
    }
    for y in H - 70..H {
        for x in 0..W {
            set(x, y, (235, 237, 241));
        }
    }

    let mut raw = Vec::with_capacity(H * (W * 3 + 1));
    for y in 0..H {
        raw.push(0);
        raw.extend_from_slice(&pixels[y * W * 3..(y + 1) * W * 3]);
    }

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(W as u32).to_be_bytes());
    ihdr.extend_from_slice(&(H as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    png_chunk(&mut out, b"IHDR", &ihdr);
    let compressed = zlib_stored(&raw);
    png_chunk(&mut out, b"IDAT", &compressed);
    png_chunk(&mut out, b"IEND", &[]);
    out
}
