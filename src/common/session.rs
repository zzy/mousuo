//! 会话持久化 — 基于 SurrealDB
//!
//! Topcoat 管理 session token 的生命周期（生成、传输、过期），
//! 应用负责将 TokenHash → 用户数据 的映射持久化到数据库。
//!
//! 表结构（SurrealDB）:
//!   session: { id: record, username: string, expires_at: int, captcha_answer: option<int>, captcha_expires_at: option<int> }

use topcoat::cookie::{Cookie, Cookies, SameSite};
use topcoat::context::Cx;
use topcoat::session::{
    SessionConfig, Token, TokenHash, TokenStore, TokenStoreFuture,
};

use crate::db;

/// TokenHash → session 记录纯 key（64 位 hex）
/// 表名只出现在 SQL 内（type::record 构造），Rust 侧零表名
pub(crate) fn encode_id(hash: &TokenHash) -> String {
    crate::common::rand::hex(hash.as_ref())
}

// ── 会话 Cookie 载体 ──────────────────────────────────────────────────────

/// 会话 Cookie 名（与 topcoat 默认一致）
const SESSION_COOKIE_NAME: &str = "session";

/// 会话 Cookie 载体：Secure 属性与 __Host- 前缀跟随请求协议
/// （x-forwarded-proto 为 https 时启用），本地 http 开发可用、生产 https 自动加固
pub struct SessionCookieStore;

impl TokenStore for SessionCookieStore {
    fn read<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, Option<Token>> {
        Box::pin(async move {
            let jar = topcoat::cookie::cookies(cx);
            let cookie = jar
                .get(SESSION_COOKIE_NAME)
                .or_else(|| jar.get(&format!("__Host-{SESSION_COOKIE_NAME}")));
            Ok(cookie.and_then(|c| Token::decode(c.value_trimmed()).ok()))
        })
    }

    fn write<'a>(
        &'a self,
        cx: &'a Cx,
        token: Token,
        max_age: std::time::Duration,
    ) -> TokenStoreFuture<'a, ()> {
        Box::pin(async move {
            let max_age = topcoat::cookie::time::Duration::try_from(max_age)?;
            // https 请求（直连或反代转发头）才启用 Secure 与 __Host- 前缀
            let secure = topcoat::router::request::headers(cx)
                .get("x-forwarded-proto")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.eq_ignore_ascii_case("https"));
            let name = if secure {
                format!("__Host-{SESSION_COOKIE_NAME}")
            } else {
                SESSION_COOKIE_NAME.to_string()
            };
            topcoat::cookie::cookies(cx)
                .override_same_site(SameSite::Lax)
                .override_http_only(true)
                .override_path("/")
                .override_max_age(max_age)
                .map(move |cookie: &mut Cookie<'static>| {
                    cookie.set_secure(secure);
                    cookie.set_name(name.clone());
                })
                .add(Cookie::new(SESSION_COOKIE_NAME, token.encode()));
            Ok(())
        })
    }

    fn delete<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, ()> {
        Box::pin(async move {
            let jar = topcoat::cookie::cookies(cx);
            jar.remove(Cookie::new(SESSION_COOKIE_NAME, ""));
            jar.remove(Cookie::new(format!("__Host-{SESSION_COOKIE_NAME}"), ""));
            Ok(())
        })
    }
}

/// 会话配置：Secure 跟随协议的 Cookie 载体
pub fn session_config() -> SessionConfig {
    SessionConfig::builder().token_store(SessionCookieStore).build()
}

/// 签发 session 时创建记录（验证码阶段可能已预建记录，此时改为更新）
pub async fn create(hash: &TokenHash, username: &str, expires_at: u64) -> Result<(), String> {
    let id = encode_id(hash);
    let db = db::get_db();
    let mut probe = db
        .query("SELECT id FROM session WHERE id = type::record('session', $id)")
        .bind(("id", id.clone()))
        .await
        .map_err(|e| e.to_string())?;
    let rows: Vec<surrealdb::types::Value> = probe.take(0).map_err(|e| e.to_string())?;
    // 登录成功即轮换 CSRF token（防会话固定）
    let token = generate_csrf_token();
    if rows.is_empty() {
        db.query(
            "CREATE session CONTENT { id: type::record('session', $id), username: $username, expires_at: $expires_at, csrf_token: $token }",
        )
        .bind(("id", id))
        .bind(("username", username.to_string()))
        .bind(("expires_at", expires_at as i64))
        .bind(("token", token))
        .await
        .map_err(|e| e.to_string())?;
    } else {
        db.query(
            "UPDATE session SET username = $username, expires_at = $expires_at, csrf_token = $token WHERE id = type::record('session', $id)",
        )
        .bind(("id", id))
        .bind(("username", username.to_string()))
        .bind(("expires_at", expires_at as i64))
        .bind(("token", token))
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 根据 token hash 解析当前用户（自动过滤过期记录；过期比较在 Rust 侧完成）
pub async fn resolve(hash: &TokenHash) -> Result<Option<String>, String> {
    let id = encode_id(hash);
    let db = db::get_db();
    let mut res = db
        .query("SELECT username, expires_at FROM session WHERE id = type::record('session', $id)")
        .bind(("id", id))
        .await
        .map_err(|e| e.to_string())?;
    let raw: Vec<surrealdb::types::Value> = res.take(0).map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let row = raw.first().and_then(|v| v.as_object());
    let expired = row
        .and_then(|obj| obj.get("expires_at"))
        .and_then(|v| db::from_value::<i64>(v))
        .is_none_or(|exp| exp <= now);
    if expired {
        return Ok(None);
    }
    let username = row
        .and_then(|obj| obj.get("username"))
        .and_then(|v| db::from_value::<String>(v));
    // 验证码阶段预建记录的 username 为空串，视为未登录
    Ok(username.filter(|u| !u.is_empty()))
}

/// 登出时删除记录
pub async fn remove(hash: &TokenHash) -> Result<(), String> {
    let id = encode_id(hash);
    let db = db::get_db();
    db.query("DELETE session WHERE id = type::record('session', $id)")
        .bind(("id", id))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除某用户的全部会话（封禁/密码重置后踢下线）
pub async fn remove_all(username: &str) -> Result<(), String> {
    let db = db::get_db();
    db.query("DELETE session WHERE username = $username")
        .bind(("username", username.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除某用户除当前会话外的全部会话（修改密码后保留当前登录）
pub async fn remove_all_except(hash: &TokenHash, username: &str) -> Result<(), String> {
    let id = encode_id(hash);
    let db = db::get_db();
    db.query("DELETE session WHERE username = $username AND id != type::record('session', $id)")
        .bind(("username", username.to_string()))
        .bind(("id", id))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}


// ── CSRF 防护 ─────────────────────────────────────────────────────────────

/// 生成 CSRF token：32 字节安全随机 → 64 位 hex
pub fn generate_csrf_token() -> String {
    crate::common::rand::random_hex()
}

/// 恒定时间比较（见 common/rand::ct_eq）
pub fn ct_eq(a: &str, b: &str) -> bool {
    crate::common::rand::ct_eq(a, b)
}

/// 确保当前会话存在 CSRF token（无则生成写入），返回 token；未签发会话返回 None
pub async fn ensure_csrf_token(cx: &Cx) -> Option<String> {
    let hash = topcoat::session::token_hash(cx).await.ok()??;
    let id = encode_id(&hash);
    let db = db::get_db();
    let mut probe = db
        .query("SELECT csrf_token FROM session WHERE id = type::record('session', $id)")
        .bind(("id", id.clone()))
        .await
        .ok()?;
    let rows: Vec<surrealdb::types::Value> = probe.take(0).ok()?;
    if let Some(token) = rows
        .first()
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("csrf_token"))
        .and_then(|v| db::from_value::<String>(v))
    {
        return Some(token);
    }
    let token = generate_csrf_token();
    let sql = if rows.is_empty() {
        "CREATE session CONTENT { id: type::record('session', $id), username: '', expires_at: 0, csrf_token: $token }"
    } else {
        "UPDATE session SET csrf_token = $token WHERE id = type::record('session', $id)"
    };
    db.query(sql)
        .bind(("id", id))
        .bind(("token", token.clone()))
        .await
        .ok()?;
    Some(token)
}

/// 校验提交的 CSRF token 与会话记录是否一致（恒定时间比较）
pub async fn verify_csrf(cx: &Cx, submitted: &str) -> bool {
    if submitted.is_empty() {
        return false;
    }
    let Some(hash) = topcoat::session::token_hash(cx).await.ok().flatten() else {
        return false;
    };
    let id = encode_id(&hash);
    let db = db::get_db();
    let mut res = match db
        .query("SELECT csrf_token FROM session WHERE id = type::record('session', $id)")
        .bind(("id", id))
        .await
    {
        Ok(r) => r,
        Err(_) => return false,
    };
    let rows: Vec<surrealdb::types::Value> = match res.take(0) {
        Ok(r) => r,
        Err(_) => return false,
    };
    rows.first()
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("csrf_token"))
        .and_then(|v| db::from_value::<String>(v))
        .is_some_and(|stored| ct_eq(&stored, submitted))
}
