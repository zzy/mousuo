//! 媒体存储 — 商品媒体上传（P3，仅管理员）
//!
//! 目录结构（data/media/，运行期生成）：
//!   图片：{key}.{ext}
//!   视频：{key}/original.{ext} + {key}/hls/playlist.m3u8 + {key}/hls/segment_*.ts
//! 访问：GET /media/{key}…（src/app/media.rs 静态服务，公开只读）
//! key 为 32 位 hex（无引号字符，与 DB key 规则一致）

use std::path::PathBuf;

use uuid::Uuid;

/// 媒体根目录
pub fn media_root() -> PathBuf {
    PathBuf::from("data/media")
}

/// 单文件上传大小上限（32MB）
pub const UPLOAD_MAX_BYTES: usize = 32 * 1024 * 1024;

/// 上传路由的 body 上限（topcoat 层）：文件上限 + multipart 边界余量
pub const UPLOAD_BODY_LIMIT: usize = UPLOAD_MAX_BYTES + 64 * 1024;

/// 图片扩展名白名单（含对应 content-type）
const IMAGE_TYPES: [(&str, &str); 4] = [
    ("jpg", "image/jpeg"),
    ("png", "image/png"),
    ("webp", "image/webp"),
    ("gif", "image/gif"),
];

/// 视频扩展名白名单
const VIDEO_TYPES: [(&str, &str); 3] = [("mp4", "video/mp4"), ("webm", "video/webm"), ("mov", "video/quicktime")];

/// MIME → 扩展名；不识别返回 None（由调用方报错）
pub fn mime_to_extension(mime: &str) -> Option<&'static str> {
    if mime.contains("jpeg") || mime.contains("jpg") {
        Some("jpg")
    } else if mime.contains("png") {
        Some("png")
    } else if mime.contains("webp") {
        Some("webp")
    } else if mime.contains("gif") {
        Some("gif")
    } else if mime.contains("mp4") {
        Some("mp4")
    } else if mime.contains("webm") {
        Some("webm")
    } else if mime.contains("quicktime") {
        Some("mov")
    } else {
        None
    }
}

/// 判断是否为图片扩展名
pub fn is_image_ext(ext: &str) -> bool {
    IMAGE_TYPES.iter().any(|(e, _)| *e == ext)
}

/// 按扩展名取 content-type（图片 + HLS 产物）
pub fn content_type_for_path(path: &PathBuf) -> &'static str {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    for (ext, ct) in IMAGE_TYPES.iter().chain(VIDEO_TYPES.iter()) {
        if name.ends_with(ext) {
            return ct;
        }
    }
    if name.ends_with(".m3u8") {
        "application/vnd.apple.mpegurl"
    } else if name.ends_with(".ts") {
        "video/mp2t"
    } else {
        "application/octet-stream"
    }
}

/// 保存图片（bytes 已校验大小），返回相对访问路径（/media/...）
pub async fn save_image(bytes: &[u8], ext: &str) -> Result<String, String> {
    let key = hex_key();
    let mut rel = String::with_capacity(key.len() + 1 + ext.len());
    rel.push_str(&key);
    rel.push('.');
    rel.push_str(ext);
    let abs = media_root().join(&rel);
    write_file(&abs, bytes).await?;
    let mut url = String::with_capacity(7 + rel.len());
    url.push_str("/media/");
    url.push_str(&rel);
    Ok(url)
}

/// 保存视频：ffprobe 校验 → HLS 转码，返回播放列表相对访问路径（/media/...）
pub async fn save_video(bytes: &[u8], ext: &str) -> Result<String, String> {
    let key = hex_key();
    let dir = media_root().join(&key);
    let mut original_name = String::with_capacity(9 + ext.len());
    original_name.push_str("original.");
    original_name.push_str(ext);
    let original = dir.join(original_name);
    write_file(&original, bytes).await?;
    crate::common::video::probe(&original)?;
    crate::common::video::transcode_to_hls(&original, &dir.join("hls"))?;
    let mut url = String::with_capacity(7 + key.len() + 16);
    url.push_str("/media/");
    url.push_str(&key);
    url.push_str("/hls/playlist.m3u8");
    Ok(url)
}

/// 校验 catch-all 段拼接出的相对路径（防目录穿越），返回拼接后的相对路径
pub fn safe_relative(segments: &[String]) -> Option<PathBuf> {
    if segments.is_empty() {
        return None;
    }
    let mut rel = PathBuf::new();
    for seg in segments {
        // 拒绝空段、`.`、`..`、含路径分隔符或反斜杠的段
        if seg.is_empty() || seg == "." || seg == ".." || seg.contains('/') || seg.contains('\\') {
            return None;
        }
        rel.push(seg);
    }
    Some(rel)
}

/// 从文本中提取全部 /media/ 引用（商品主图与描述内嵌媒体共用），
/// 只认 32 位小写 hex key（与存储规则一致，防误匹配/误删）
pub fn extract_media_urls(text: &str) -> Vec<&str> {
    let mut urls = Vec::new();
    let mut rest = text;
    while let Some(idx) = rest.find("/media/") {
        let after = &rest[idx + 7..];
        // 收集到第一个非 [0-9a-f] 的字符，仅当恰好 32 位小写 hex 才视为有效引用
        let hex_len = after
            .chars()
            .take_while(|c| c.is_ascii_digit() || ('a'..='f').contains(c))
            .count();
        if hex_len == 32 {
            urls.push(&rest[idx..idx + 7 + 32]);
        }
        rest = &after[hex_len.min(after.len())..];
    }
    urls
}

/// 回滚已保存的媒体（建记录失败时清理）：/media/{key}… → 删除对应文件或目录
pub async fn remove_upload(url: &str) -> Result<(), String> {
    let Some(rest) = url.strip_prefix("/media/") else {
        return Err("非媒体地址".to_string());
    };
    let rel = PathBuf::from(rest);
    if rel.components().count() == 0 || rel.file_name().is_none() {
        return Err("非法媒体地址".to_string());
    }
    let abs = media_root().join(&rel);
    // 图片：{key}.{ext}；视频：{key}/hls/… → 目标为 key 目录（rel 的首段）
    let target = match abs.extension() {
        Some(_) => abs.clone(),
        None => rel
            .components()
            .next()
            .map(|c| media_root().join(c.as_os_str()))
            .unwrap_or(abs),
    };
    if target.is_dir() {
        tokio::fs::remove_dir_all(&target)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        tokio::fs::remove_file(&target)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn hex_key() -> String {
    Uuid::new_v4().simple().to_string()
}

async fn write_file(path: &PathBuf, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("创建媒体目录失败: {e}"))?;
    }
    tokio::fs::write(path, bytes)
        .await
        .map_err(|e| format!("写入媒体文件失败: {e}"))
}
