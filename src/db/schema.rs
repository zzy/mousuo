use crate::db;

/// 启动时同步表结构（幂等，遵循 SurrealDB 规范显式定义）
pub async fn ensure_tables() -> Result<(), String> {
    let db = db::get_db();
    // user 用户表
    db.query(
        "DEFINE TABLE IF NOT EXISTS user SCHEMAFULL;
         DEFINE FIELD IF NOT EXISTS username ON user TYPE string;
         DEFINE FIELD IF NOT EXISTS cred ON user TYPE string;
         DEFINE FIELD IF NOT EXISTS email ON user TYPE string;
         DEFINE FIELD IF NOT EXISTS introduction ON user TYPE string;
         DEFINE FIELD IF NOT EXISTS topics ON user TYPE string;
         DEFINE FIELD IF NOT EXISTS status ON user TYPE int;
         DEFINE FIELD IF NOT EXISTS activation_token ON user TYPE option<string>;
         DEFINE FIELD IF NOT EXISTS password_reset_token ON user TYPE option<string>;
         DEFINE FIELD IF NOT EXISTS password_reset_expires_at ON user TYPE option<int>;
         DEFINE FIELD IF NOT EXISTS is_admin ON user TYPE int DEFAULT 0;
         UPDATE user SET is_admin = 0 WHERE is_admin = NONE;",
    )
    .await
    .map_err(|e| e.to_string())?;
    // session 会话表（captcha 字段供登录验证码使用）
    db.query(
        "DEFINE TABLE IF NOT EXISTS session SCHEMAFULL;
         DEFINE FIELD IF NOT EXISTS username ON session TYPE string;
         DEFINE FIELD IF NOT EXISTS expires_at ON session TYPE int;
         DEFINE FIELD IF NOT EXISTS captcha_answer ON session TYPE option<int>;
         DEFINE FIELD IF NOT EXISTS captcha_expires_at ON session TYPE option<int>;
         DEFINE FIELD IF NOT EXISTS csrf_token ON session TYPE option<string>;",
    )
    .await
    .map_err(|e| e.to_string())?;
    // arc 内容表
    db.query(
        "DEFINE TABLE IF NOT EXISTS arc SCHEMAFULL;
         DEFINE FIELD IF NOT EXISTS title ON arc TYPE string;
         DEFINE FIELD IF NOT EXISTS slug ON arc TYPE string;
         DEFINE FIELD IF NOT EXISTS arc_type ON arc TYPE string;
         DEFINE FIELD IF NOT EXISTS body ON arc TYPE option<string>;
         DEFINE FIELD IF NOT EXISTS summary ON arc TYPE option<string>;
         DEFINE FIELD IF NOT EXISTS thumbnail ON arc TYPE option<string>;
         DEFINE FIELD IF NOT EXISTS media_url ON arc TYPE option<string>;
         DEFINE FIELD IF NOT EXISTS author_name ON arc TYPE option<string>;
         DEFINE FIELD IF NOT EXISTS topics ON arc TYPE option<string>;
         DEFINE FIELD IF NOT EXISTS created_at ON arc TYPE datetime;
         DEFINE FIELD IF NOT EXISTS view_count ON arc TYPE int;",
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
