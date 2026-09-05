use crate::common::config;
use serde::de::DeserializeOwned;
use std::sync::OnceLock;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::types::Value;
use uuid::Uuid;

pub mod orders;
pub mod products;
pub mod schema;
pub mod users;

static DB: OnceLock<Surreal<Client>> = OnceLock::new();

pub async fn init() {
    let cfg = config::config();
    let db = Surreal::new::<Ws>(&cfg.db_url)
        .await
        .unwrap_or_else(|e| panic!("connect {}: {e}", cfg.db_url));

    db.signin(Root {
        username: cfg.db_user.clone(),
        password: cfg.db_pass.clone(),
    })
    .await
    .unwrap_or_else(|e| panic!("auth: {e}"));

    db.use_ns(&cfg.db_ns)
        .await
        .unwrap_or_else(|e| panic!("ns: {e}"));
    db.use_db(&cfg.db_name)
        .await
        .unwrap_or_else(|e| panic!("db: {e}"));

    DB.set(db).expect("DB already set");

    get_db()
        .query("RETURN 1")
        .await
        .unwrap_or_else(|e| panic!("db health check: {e}"));

    eprintln!("  SurrealDB connected {}/{}/{}", cfg.db_url, cfg.db_ns, cfg.db_name);
}

pub fn get_db() -> &'static Surreal<Client> {
    DB.get().expect("db::init() not called")
}

// ── 通用查询抽象 ──────────────────────────────────────────────────────────

pub(crate) fn from_value<T: DeserializeOwned>(v: &Value) -> Option<T> {
    // SurrealDB 3.x 的 Value 直接 serde 序列化是枚举结构体，
    // 必须先经 into_json_value 转为标准 JSON 再反序列化进模型
    serde_json::from_value(v.clone().into_json_value()).ok()
}

/// 生成新记录 key：32 位 hex（官方 ID_CHARS 安全字符集）
pub fn new_record_key() -> String {
    Uuid::new_v4().simple().to_string()
}

pub async fn query_as<T: DeserializeOwned>(
    sql: &str,
    params: &[(&str, Value)],
) -> Result<Vec<T>, String> {
    let db = get_db();
    let mut sql_query = db.query(sql);
    for (k, v) in params {
        sql_query = sql_query.bind((*k, v.clone()));
    }
    let mut res = sql_query.await.map_err(|e| e.to_string())?;
    let raw: Vec<Value> = res.take(0).map_err(|e| e.to_string())?;
    Ok(raw.iter().filter_map(|v| from_value(v)).collect())
}

pub async fn query_one<T: DeserializeOwned>(
    sql: &str,
    params: &[(&str, Value)],
) -> Result<Option<T>, String> {
    query_as(sql, params).await.map(|v| v.into_iter().next())
}
