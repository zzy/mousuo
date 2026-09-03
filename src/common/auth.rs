//! 认证模块 — 基于 Topcoat Session
//!
//! 登录时通过 session::start() 签发 session token，
//! 将 token hash → username 的映射存入 SurrealDB；
//! 后续请求通过 session::token_hash() + DB 查询解析用户身份。
//! 登出时删除 session 记录并清除 cookie。

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use topcoat::context::Cx;

use crate::common::session as session_db;

/// Argon2id 哈希凭证（PHC 字符串，含随机盐）
pub fn hash_credential(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Argon2 哈希失败")
        .to_string()
}

/// 密码强度：须同时含大写、小写、数字（注册/修改/重置共用）
pub fn password_strong(password: &str) -> bool {
    password.chars().any(|c| c.is_uppercase())
        && password.chars().any(|c| c.is_lowercase())
        && password.chars().any(|c| c.is_ascii_digit())
}

/// 校验凭证
pub fn verify_credential(password: &str, stored_cred: &str) -> bool {
    PasswordHash::new(stored_cred).ok().map_or(false, |h| {
        Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}

/// 登录：创建 session → DB 写入 (hash, username, expires_at)
pub async fn sign_in(cx: &Cx, username: &str) -> Result<(), String> {
    let sess = topcoat::session::start(cx)
        .await
        .map_err(|e| e.to_string())?;
    let expires_at = sess
        .expires_at
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    session_db::create(&sess.token_hash, username, expires_at).await
}

/// 登出：删除 session 记录 + 清除 cookie
pub async fn sign_out(cx: &Cx) -> Result<(), String> {
    if let Some(hash) = topcoat::session::stop(cx)
        .await
        .map_err(|e| e.to_string())?
    {
        session_db::remove(&hash).await?;
    }
    Ok(())
}

/// 从当前请求的 session 中获取登录用户名
pub async fn current_user(cx: &Cx) -> Option<String> {
    let hash = topcoat::session::token_hash(cx).await.ok()??;
    session_db::resolve(&hash).await.ok().flatten()
}

/// 当前请求是否为管理员（P3：is_admin 标记，试水不做 RBAC）
pub async fn is_admin(cx: &Cx) -> bool {
    match current_user(cx).await {
        Some(username) => crate::db::users::find_user(&username)
            .await
            .map(|u| u.is_some_and(|u| u.is_admin == crate::common::constant::USER_IS_ADMIN))
            .unwrap_or(false),
        None => false,
    }
}
