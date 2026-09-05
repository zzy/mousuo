use crate::i18n::loader;
use topcoat::context::Cx;
use topcoat::router::{Body, StatusCode, header, response::Response};

/// 解析 URL 中的指定查询参数值
pub fn query_param(cx: &Cx, key: &str) -> Option<String> {
    let parts = topcoat::router::request::parts(cx);
    parts.uri.query().and_then(|query| {
        query.split('&').find_map(|p| {
            p.split_once('=')
                .and_then(|(k, v)| if k == key { Some(v.to_string()) } else { None })
        })
    })
}

/// 解析 URL 中的 ?error= 参数，返回对应 i18n 消息
pub fn error_message(cx: &Cx, locale: &str, keys: &[&str]) -> Option<String> {
    let parts = topcoat::router::request::parts(cx);
    let error_key = parts.uri.query().and_then(|query| {
        query
            .split('&')
            .find_map(|p| p.strip_prefix("error=").map(|v| v.to_string()))
    });
    let err = error_key.as_deref()?;
    if !keys.contains(&err) {
        return None;
    }
    let i18n_key = crate::common::constant::error_i18n_key(err)?;
    Some(loader::t(locale, i18n_key).to_string())
}

/// 回跳路径校验：仅允许站内相对路径（防开放重定向与响应头注入）
pub fn safe_next(next: &str) -> Option<String> {
    if !next.starts_with('/') || next.starts_with("//") {
        return None;
    }
    if !next.bytes().all(|b| (0x20..0x7e).contains(&b)) {
        return None;
    }
    Some(next.to_string())
}

/// 重定向响应
pub fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, location)
        .body(Body::empty())
        .unwrap()
}
