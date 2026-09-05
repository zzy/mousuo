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
    // product 商品表（字段类型见 DESIGN.md 三、数据模型）
    db.query(
        "DEFINE TABLE IF NOT EXISTS product SCHEMAFULL;
         DEFINE FIELD IF NOT EXISTS slug ON product TYPE string;
         DEFINE FIELD IF NOT EXISTS title ON product TYPE string;
         DEFINE FIELD IF NOT EXISTS description ON product TYPE string;
         DEFINE FIELD IF NOT EXISTS price_cents ON product TYPE int;
         DEFINE FIELD IF NOT EXISTS stock ON product TYPE int;
         DEFINE FIELD IF NOT EXISTS image ON product TYPE option<string>;
         DEFINE FIELD IF NOT EXISTS status ON product TYPE string;
         DEFINE FIELD IF NOT EXISTS created_at ON product TYPE datetime;",
    )
    .await
    .map_err(|e| e.to_string())?;
    // order 订单表（字段类型见 DESIGN.md 三、数据模型）
    // items 为对象数组：需 FLEXIBLE 允许嵌套字段（3.2 实测 TYPE array 会拒收对象）；
    // FLEXIBLE 会自动派生 items.* 字段（非 FLEXIBLE）并在校验时覆盖父字段标记，
    // 必须 OVERWRITE 补为 FLEXIBLE；OVERWRITE 幂等且不抹数据，可保证新旧库收敛
    db.query(
        "DEFINE TABLE IF NOT EXISTS order SCHEMAFULL;
         DEFINE FIELD IF NOT EXISTS user_id ON order TYPE string;
         DEFINE FIELD OVERWRITE items ON order TYPE array<object> FLEXIBLE;
         DEFINE FIELD OVERWRITE items.* ON order TYPE object FLEXIBLE;
         DEFINE FIELD IF NOT EXISTS total_cents ON order TYPE int;
         DEFINE FIELD IF NOT EXISTS status ON order TYPE string;
         DEFINE FIELD IF NOT EXISTS stripe_session_id ON order TYPE option<string>;
         DEFINE FIELD IF NOT EXISTS paid_at ON order TYPE option<datetime>;
         DEFINE FIELD IF NOT EXISTS cancelled_at ON order TYPE option<datetime>;
         DEFINE FIELD IF NOT EXISTS created_at ON order TYPE datetime;",
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
