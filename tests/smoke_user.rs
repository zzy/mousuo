//! 冒烟前置：幂等创建已激活测试账号 smoketester（需 DB 连接，显式执行）
//!
//! 运行方式：`cargo test --test smoke_user -- --ignored`

use mousuo::common::auth;
use mousuo::db;
use surrealdb::types::SurrealValue;

#[tokio::test]
#[ignore = "需要 SurrealDB 连接，显式运行"]
async fn create_smoke_user_idempotently() {
    dotenvy::dotenv().ok();
    db::init().await;
    let exists: Option<surrealdb::types::Value> = db::query_one(
        "SELECT username FROM user WHERE username = $user",
        &[("user", "smoketester".to_string().into_value())],
    )
    .await
    .ok()
    .flatten();
    if exists.is_some() {
        return; // 已存在，幂等跳过
    }
    let cred = auth::hash_credential("Smoke123a");
    db::get_db()
        .query(
            "CREATE user CONTENT { username: $user, cred: $cred, email: 'smoke@test.local', introduction: '', status: 1 }",
        )
        .bind(("user", "smoketester".to_string()))
        .bind(("cred", cred))
        .await
        .expect("创建测试账号失败");
}
