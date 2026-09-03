#![allow(non_snake_case)]

use crate::common::media;
use topcoat::{
    Result,
    context::Cx,
    router::{Body, StatusCode, header, path_param_segments, response::Response},
};

/// 上传媒体静态服务：GET /media/{*rest} → data/media/{rest}（公开只读）
/// 只允许列白名单的媒体产物类型，拒绝目录穿越
#[topcoat::router::route(GET "/media/{*rest}")]
pub async fn media_file(cx: &Cx) -> Result<Response> {
    let segments: Vec<String> = path_param_segments(cx, "rest").map(|s| s.to_string()).collect();
    let Some(rel) = media::safe_relative(&segments) else {
        return Ok(not_found());
    };
    let abs = media::media_root().join(&rel);
    let bytes = match tokio::fs::read(&abs).await {
        Ok(b) => b,
        Err(_) => return Ok(not_found()),
    };
    let content_type = media::content_type_for_path(&rel);
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .body(Body::from(bytes))
        .expect("构建媒体响应失败"))
}

fn not_found() -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::empty())
        .expect("构建媒体 404 响应失败")
}
