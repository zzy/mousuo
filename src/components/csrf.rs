#![allow(non_snake_case)]

use topcoat::{
    Result,
    view::{View, component, view},
};

/// CSRF 隐藏域 — 所有状态变更 POST 表单必须携带（值来自 session::ensure_csrf_token）
#[component]
pub async fn CsrfField(token: String) -> Result<impl View> {
    Ok(view! { <input type="hidden" name="csrf_token" value=(token)> })
}
