use crate::common::auth;
use crate::common::constant::{USER_STATUS_ACTIVE, USER_STATUS_PENDING};
use crate::db;
use crate::models::user::User;
use surrealdb::types::SurrealValue;

/// 注册用户（状态为待激活）；返回 (用户, 激活 token，防激活劫持不暴露 id)
pub async fn register_user(
    username: &str,
    password: &str,
    email: &str,
    introduction: &str,
) -> Result<(User, String), String> {
    let cred = auth::hash_credential(password);
    let token = crate::common::rand::random_hex();
    let mut res = db::get_db()
        .query("CREATE user CONTENT { username: $user, cred: $cred, email: $email, introduction: $intro, status: $status, activation_token: $tok } RETURN id, username, cred, email, introduction, status")
        .bind(("user", username.to_string()))
        .bind(("cred", cred))
        .bind(("email", email.to_string()))
        .bind(("intro", introduction.to_string()))
        .bind(("status", USER_STATUS_PENDING))
        .bind(("tok", token.clone()))
        .await
        .map_err(|e| e.to_string())?;
    let raw: Vec<surrealdb::types::Value> = res.take(0).map_err(|e| e.to_string())?;
    let user = raw
        .iter()
        .filter_map(db::from_value)
        .next()
        .ok_or_else(|| "注册失败".to_string())?;
    Ok((user, token))
}

/// 按激活 token 激活用户：条件原子更新（token 不匹配则 0 行），成功后清 token 防重放
pub async fn activate_by_token(token: &str) -> Result<(), String> {
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("激活链接无效或已过期".to_string());
    }
    let db = db::get_db();
    let mut res = db
        .query(
            "UPDATE user SET status = $status, activation_token = NONE WHERE activation_token = $tok",
        )
        .bind(("status", USER_STATUS_ACTIVE as i64))
        .bind(("tok", token.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    // 单记录 UPDATE 返回是对象而非数组，用 Vec<Value> 读取（见 db_id_binding 回归）
    let updated: Vec<surrealdb::types::Value> = res.take(0).map_err(|e| e.to_string())?;
    if updated.is_empty() {
        return Err("激活链接无效或已过期".to_string());
    }
    Ok(())
}

pub async fn find_user(username: &str) -> Result<Option<User>, String> {
    db::query_one(
        "SELECT id, username, cred, email, introduction, status, is_admin FROM user WHERE username = $user",
        &[("user", username.to_string().into_value())],
    )
    .await
}

/// 按用户名或邮箱查找
pub async fn find_by_account(account: &str) -> Result<Option<User>, String> {
    if account.contains('@') {
        db::query_one(
            "SELECT id, username, cred, email, introduction, status, is_admin FROM user WHERE email = $sig",
            &[("sig", account.to_string().into_value())],
        )
        .await
    } else {
        find_user(account).await
    }
}

/// 按用户 id 查找（管理端封禁用）
pub async fn find_user_by_id(user_id: &str) -> Result<Option<User>, String> {
    db::query_one(
        "SELECT id, username, cred, email, introduction, status, is_admin FROM user WHERE id = $id",
        &[("id", db::record_id(user_id)?)],
    )
    .await
}

/// 按用户名查找用户公开信息（资料页）
pub async fn get_user_profile(username: &str) -> Result<Option<User>, String> {
    db::query_one(
        "SELECT id, username, cred, email, introduction, status FROM user WHERE username = $user",
        &[("user", username.to_string().into_value())],
    )
    .await
}

/// 设置用户状态（管理端封禁/解封）
pub async fn set_user_status(user_id: &str, status: u8) -> Result<(), String> {
    let db = db::get_db();
    db.query("UPDATE $id SET status = $status")
        .bind(("id", db::record_id(user_id)?))
        .bind(("status", status as i64))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 轮换激活 token（重发激活邮件）：仅待激活账户可重发；返回 (邮箱, 新 token)
pub async fn rotate_activation_token(account: &str) -> Result<(String, String), String> {
    let user = find_by_account(account).await?.ok_or("账户不存在".to_string())?;
    if user.status != crate::common::constant::USER_STATUS_PENDING {
        return Err("账户无需激活".to_string());
    }
    let token = crate::common::rand::random_hex();
    db::get_db()
        .query("UPDATE $id SET activation_token = $tok")
        .bind(("id", db::record_id(&user.id)?))
        .bind(("tok", token.clone()))
        .await
        .map_err(|e| e.to_string())?;
    Ok((user.email, token))
}

/// 更新密码哈希（修改/重置密码共用）
pub async fn update_cred(username: &str, cred: &str) -> Result<(), String> {
    db::get_db()
        .query("UPDATE user SET cred = $cred WHERE username = $user")
        .bind(("cred", cred.to_string()))
        .bind(("user", username.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 生成密码重置 token（1 小时有效）；返回 (邮箱, token)，供邮件发送
pub async fn issue_password_reset(account: &str) -> Result<(String, String), String> {
    let user = find_by_account(account).await?.ok_or("账户不存在".to_string())?;
    let token = crate::common::rand::random_hex();
    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        + crate::common::constant::PASSWORD_RESET_EXPIRY as i64;
    db::get_db()
        .query("UPDATE $id SET password_reset_token = $tok, password_reset_expires_at = $exp")
        .bind(("id", db::record_id(&user.id)?))
        .bind(("tok", token.clone()))
        .bind(("exp", expires))
        .await
        .map_err(|e| e.to_string())?;
    Ok((user.email, token))
}

/// 完成密码重置：token 有效且未过期 → 更新 cred + 清 token；返回 username（供踢会话）
pub async fn complete_password_reset(token: &str, cred: &str) -> Result<String, String> {
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("重置链接无效或已过期".to_string());
    }
    let db = db::get_db();
    let mut res = db
        .query("SELECT username, password_reset_expires_at FROM user WHERE password_reset_token = $tok")
        .bind(("tok", token.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    let rows: Vec<surrealdb::types::Value> = res.take(0).map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let username = rows
        .first()
        .and_then(|v| v.as_object())
        .and_then(|obj| {
            let expired = obj
                .get("password_reset_expires_at")
                .and_then(|v| db::from_value::<i64>(v))
                .is_none_or(|exp| exp <= now);
            if expired {
                None
            } else {
                obj.get("username").and_then(|v| db::from_value::<String>(v))
            }
        })
        .ok_or("重置链接无效或已过期".to_string())?;
    db.query("UPDATE user SET cred = $cred, password_reset_token = NONE, password_reset_expires_at = NONE WHERE username = $user")
        .bind(("cred", cred.to_string()))
        .bind(("user", username.clone()))
        .await
        .map_err(|e| e.to_string())?;
    Ok(username)
}

/// 设置管理员标记（管理端任免；不可取消自己由调用方保证）
pub async fn set_user_is_admin(user_id: &str, is_admin: u8) -> Result<(), String> {
    db::get_db()
        .query("UPDATE $id SET is_admin = $flag")
        .bind(("id", db::record_id(user_id)?))
        .bind(("flag", is_admin as i64))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 用户分页列表（管理端；cred 仅供反序列化完整模型，视图层不得展示）
pub async fn list_users(page: u64, page_size: u64) -> Result<Vec<User>, String> {
    let start = ((page - 1) * page_size) as i64;
    db::query_as(
        "SELECT id, username, cred, email, introduction, status, is_admin FROM user ORDER BY username ASC LIMIT $limit START $start",
        &[
            ("limit", (page_size as i64).into_value()),
            ("start", start.into_value()),
        ],
    )
    .await
}

/// 用户总数
pub async fn count_users() -> Result<u64, String> {
    let db = db::get_db();
    let mut res = db
        .query("SELECT count() FROM user GROUP ALL")
        .await
        .map_err(|e| e.to_string())?;
    let count: Option<u64> = res.take((0, "count")).map_err(|e| e.to_string())?;
    Ok(count.unwrap_or(0))
}
