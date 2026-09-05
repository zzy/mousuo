//! 管理域 — 路由均受 AdminGuard 层保护（未登录 302、非管理员 404）

pub mod orders;
pub mod products;
pub mod upload;
pub mod users;

use crate::i18n::loader;
use topcoat::context::Cx;

/// 管理端操作结果横幅：解析 ?ok= 参数并按映射表取 i18n 文案
pub fn notice(cx: &Cx, locale: &str, keys: &[(&str, &str)]) -> Option<String> {
    let raw = crate::common::form::query_param(cx, "ok")?;
    keys.iter()
        .find(|(k, _)| *k == raw)
        .map(|(_, key)| loader::t(locale, key).to_string())
}
