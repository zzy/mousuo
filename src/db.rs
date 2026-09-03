use crate::common::config;
use serde::de::DeserializeOwned;
use std::sync::OnceLock;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::types::{SurrealValue, Value};
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

/// 将 record id 全形字符串（如 session:abc123）转为可绑定的 RecordId 值
///
/// 远端 3.2.4 实测语义（勿再踩）：
/// - `id = $id` 比较绑定裸字符串：静默不匹配
/// - `UPDATE $id` 绑定裸字符串：运行时报错 Cannot execute UPDATE statement
/// - `CREATE ... CONTENT { id: $id }` 绑定裸字符串：整串当 key 写入
/// - 正确形态：统一绑定本函数产出的 RecordId 值
///
/// 前提：key 只含 [0-9a-zA-Z_]（无 ⟨⟩/反引号/连字符），
/// 否则全形字符串带引号，parse_simple 无法往返；
/// 新 id 一律用 new_record_id / session hex key 生成。
pub fn record_id(id: &str) -> Result<Value, String> {
    surrealdb::types::RecordId::parse_simple(id)
        .map(|rid| rid.into_value())
        .map_err(|e| e.to_string())
}

/// 生成新 record id 全形字符串：table + 32 位 hex（无引号字符，可安全往返）
/// 返回 `{table}:{uuid hex}`，绑定查询时再经 record_id 转 RecordId 值
pub fn new_record_id(table: &str) -> String {
    let mut id = String::with_capacity(table.len() + 33);
    id.push_str(table);
    id.push(':');
    id.push_str(&Uuid::new_v4().simple().to_string());
    id
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
