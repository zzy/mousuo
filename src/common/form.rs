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
    let i18n_key = match err {
        "captcha" => "captcha_invalid",
        "incorrect" => "sign_in_incorrect",
        "not_activation" => "sign_in_not_activation",
        "banned" => "sign_in_banned",
        "security" => "sign_in_security_problem",
        "password_weak" => "register_password_weak",
        "password_mismatch" => "register_password_mismatch",
        "exist" => "register_exist",
        "payment" => "checkout_failed",
        "invalid" => "checkout_invalid",
        "stock" => "product_out_of_stock",
        "slug_invalid" => "admin_form_slug_invalid",
        "slug_exists" => "admin_form_slug_exists",
        "title_invalid" => "admin_form_title_invalid",
        "description_invalid" => "admin_form_description_invalid",
        "price_invalid" => "admin_form_price_invalid",
        "stock_invalid" => "admin_form_stock_invalid",
        "create_failed" => "admin_form_create_failed",
        "update_failed" => "admin_form_update_failed",
        "upload_empty" => "admin_upload_empty",
        "upload_too_large" => "admin_upload_too_large",
        "upload_type" => "admin_upload_type_invalid",
        "upload_failed" => "admin_upload_failed",
        _ => return None,
    };
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
