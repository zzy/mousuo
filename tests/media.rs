//! 媒体模块纯函数测试（MIME 白名单、content-type、目录穿越防护、媒体引用提取）

use mousuo::common::media::{
    content_type_for_path, extract_media_urls, is_image_ext, mime_to_extension, safe_relative,
};
use std::path::PathBuf;

#[test]
fn mime_maps_to_whitelisted_extensions() {
    assert_eq!(mime_to_extension("image/jpeg"), Some("jpg"));
    assert_eq!(mime_to_extension("image/jpg"), Some("jpg"));
    assert_eq!(mime_to_extension("image/png"), Some("png"));
    assert_eq!(mime_to_extension("image/webp"), Some("webp"));
    assert_eq!(mime_to_extension("image/gif"), Some("gif"));
    assert_eq!(mime_to_extension("video/mp4"), Some("mp4"));
    assert_eq!(mime_to_extension("video/webm"), Some("webm"));
    assert_eq!(mime_to_extension("video/quicktime"), Some("mov"));
}

#[test]
fn unknown_mimes_are_rejected() {
    assert_eq!(mime_to_extension("text/plain"), None);
    assert_eq!(mime_to_extension("application/pdf"), None);
    assert_eq!(mime_to_extension(""), None);
}

#[test]
fn image_ext_classification() {
    for ext in ["jpg", "png", "webp", "gif"] {
        assert!(is_image_ext(ext), "{ext} 应为图片");
    }
    for ext in ["mp4", "webm", "mov", "txt", ""] {
        assert!(!is_image_ext(ext), "{ext} 不应为图片");
    }
}

#[test]
fn content_types_by_extension() {
    assert_eq!(content_type_for_path(&PathBuf::from("a.png")), "image/png");
    assert_eq!(content_type_for_path(&PathBuf::from("a.jpg")), "image/jpeg");
    assert_eq!(
        content_type_for_path(&PathBuf::from("playlist.m3u8")),
        "application/vnd.apple.mpegurl"
    );
    assert_eq!(
        content_type_for_path(&PathBuf::from("segment_000.ts")),
        "video/mp2t"
    );
    assert_eq!(
        content_type_for_path(&PathBuf::from("unknown.xyz")),
        "application/octet-stream"
    );
}

#[test]
fn safe_relative_accepts_plain_segments() {
    let ok = safe_relative(&["a".to_string(), "b.png".to_string()]);
    assert_eq!(ok, Some(PathBuf::from("a").join("b.png")));
    let ok = safe_relative(&["only.png".to_string()]);
    assert_eq!(ok, Some(PathBuf::from("only.png")));
}

#[test]
fn safe_relative_rejects_traversal_and_weird_segments() {
    for segments in [
        vec![],
        vec!["".to_string()],
        vec![".".to_string()],
        vec!["..".to_string()],
        vec!["a".to_string(), "..".to_string(), "b".to_string()],
        vec!["a/b".to_string()],
        vec!["a\\b".to_string()],
        vec!["..\\x".to_string()],
    ] {
        assert_eq!(safe_relative(&segments), None, "应拒绝: {segments:?}");
    }
}

#[test]
fn extract_media_urls_finds_all_32hex_refs() {
    let text = "主图 /media/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.png 描述 <video src=\"/media/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb/hls/playlist.m3u8\"> 与 /media/cccccccccccccccccccccccccccccccc.png";
    let urls = extract_media_urls(text);
    assert_eq!(urls.len(), 3);
    assert_eq!(urls[0], "/media/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert_eq!(urls[1], "/media/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    assert_eq!(urls[2], "/media/cccccccccccccccccccccccccccccccc");
}

#[test]
fn extract_media_urls_ignores_invalid_refs() {
    // 非 32 位 hex、大写、越界长度均不匹配
    let text = "/media/abc /media/ABCABCABCABCABCABCABCABCABCABCAB /media/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa /media/";
    assert!(extract_media_urls(text).is_empty());
    assert!(extract_media_urls("").is_empty());
    assert!(extract_media_urls("无媒体").is_empty());
}
