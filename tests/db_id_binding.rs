//! SurrealDB 3.x id 语义回归测试（需 DB 连接，显式执行）
//!
//! 运行方式：`cargo test --test db_id_binding -- --ignored`
//!
//! 远端 3.2.4 实测语义（本项目最大坑，勿再踩）：
//! - `id = $id` 比较绑定裸字符串 → 静默不匹配
//! - `UPDATE $id` 绑定裸字符串 → 运行时报错 Cannot execute UPDATE statement
//! - `CREATE ... CONTENT { id: $id }` 绑定裸字符串 → 整串当 key 写入
//! - 正确形态：SQL 内以官方函数 `type::record('表名', $id)` 构造 record id，
//!   Rust 侧只绑定纯 key 字符串（表名零散落）
//! - 全链路 key 为纯 key：查询经 `id.id()` 取纯 key，生成走 `db::new_record_key()`
//! - 单记录 UPDATE 的返回是对象而非数组：读取必须用 Vec<Value>，
//!   Option<Vec<Value>> 会报 Expected array<any>, got object

use mousuo::db;
use surrealdb::types::Value;

const KEY: &str = "abc123";

#[tokio::test]
#[ignore = "需要 SurrealDB 连接，显式运行"]
async fn record_id_binding_semantics() {
    dotenvy::dotenv().ok();
    db::init().await;
    let db = db::get_db();

    // 幂等：先清划痕表，保证可重复运行
    let _ = db.query("REMOVE TABLE IF EXISTS probe_tmp").await;

    // CREATE CONTENT 以 type::record 构造 → key 精确写入
    let mut res = db
        .query("CREATE probe_tmp CONTENT { id: type::record('probe_tmp', $id), x: 1 }")
        .bind(("id", KEY.to_string()))
        .await
        .expect("create");
    let rows: Vec<Value> = res.take(0).expect("take");
    let id = rows.first().and_then(|v| v.as_object())
        .and_then(|o| o.get("id"))
        .and_then(|v| v.as_record().cloned())
        .expect("create 返回含 record id");
    assert_eq!(id.table.as_str(), "probe_tmp");
    assert!(
        matches!(&id.key, surrealdb::types::RecordIdKey::String(s) if s == KEY),
        "CREATE 的 key 应为 abc123，实际 {:?}",
        id.key
    );

    // SELECT WHERE id = type::record → 命中
    let mut res = db
        .query("SELECT x FROM probe_tmp WHERE id = type::record('probe_tmp', $id)")
        .bind(("id", KEY.to_string()))
        .await
        .expect("select");
    let rows: Vec<Value> = res.take(0).expect("take");
    assert_eq!(rows.len(), 1, "type::record SELECT 应命中 1 行");

    // UPDATE type::record(...) SET → 更新成功且返回更新行
    let mut res = db
        .query("UPDATE type::record('probe_tmp', $id) SET x = 2")
        .bind(("id", KEY.to_string()))
        .await
        .expect("update-direct");
    let rows: Vec<Value> = res.take(0).expect("take");
    assert_eq!(rows.len(), 1, "UPDATE type::record 应更新 1 行");

    // UPDATE ... WHERE id = type::record → 更新成功
    let mut res = db
        .query("UPDATE probe_tmp SET x = 3 WHERE id = type::record('probe_tmp', $id)")
        .bind(("id", KEY.to_string()))
        .await
        .expect("update-where");
    let rows: Vec<Value> = res.take(0).expect("take");
    assert_eq!(rows.len(), 1, "UPDATE WHERE type::record 应更新 1 行");

    // 复查值
    let mut res = db
        .query("SELECT x FROM probe_tmp WHERE id = type::record('probe_tmp', $id)")
        .bind(("id", KEY.to_string()))
        .await
        .expect("select2");
    let rows: Vec<Value> = res.take(0).expect("take");
    let x = rows.first().and_then(|v| v.as_object())
        .and_then(|o| o.get("x"))
        .and_then(|v| v.as_int().copied());
    assert_eq!(x, Some(3), "UPDATE 后 SELECT 应读到新值");

    // UPDATE type::record + WHERE 不命中 → 空 rows（扣库存失败分支依赖此语义）
    let mut res = db
        .query("UPDATE type::record('probe_tmp', $id) SET x = 5 WHERE x > 100")
        .bind(("id", KEY.to_string()))
        .await
        .expect("update-miss");
    let rows: Vec<Value> = res.take(0).expect("take");
    assert!(rows.is_empty(), "WHERE 不命中时 UPDATE 应返回空 rows");

    // DELETE WHERE id = type::record → 真删除
    let _ = db
        .query("DELETE probe_tmp WHERE id = type::record('probe_tmp', $id)")
        .bind(("id", KEY.to_string()))
        .await
        .expect("delete");
    let mut res = db
        .query("SELECT x FROM probe_tmp WHERE id = type::record('probe_tmp', $id)")
        .bind(("id", KEY.to_string()))
        .await
        .expect("select-after-delete");
    let rows: Vec<Value> = res.take(0).expect("take");
    assert!(rows.is_empty(), "DELETE 后 SELECT 应无行");

    // 对照：裸字符串绑定的三种错误形态（文档化，防回退）
    let mut res = db
        .query("CREATE probe_tmp CONTENT { id: $id, x: 9 }")
        .bind(("id", "str99".to_string()))
        .await
        .expect("create-str");
    let rows: Vec<Value> = res.take(0).expect("take");
    let id = rows.first().and_then(|v| v.as_object())
        .and_then(|o| o.get("id"))
        .and_then(|v| v.as_record().cloned())
        .expect("create-str 返回含 record id");
    assert!(
        matches!(&id.key, surrealdb::types::RecordIdKey::String(s) if s == "str99"),
        "裸字符串 CREATE 会把整串当 key，实际 {:?}",
        id.key
    );

    let mut res = db
        .query("UPDATE $id SET x = 10")
        .bind(("id", "str99".to_string()))
        .await
        .expect("update-dollar-str");
    let taken: surrealdb::Result<Vec<Value>> = res.take(0);
    assert!(taken.is_err(), "裸字符串 UPDATE $id 应报错");

    let mut res = db
        .query("SELECT x FROM probe_tmp WHERE id = $id")
        .bind(("id", "str99".to_string()))
        .await
        .expect("select-str");
    let rows: Vec<Value> = res.take(0).expect("take");
    assert!(rows.is_empty(), "裸字符串 SELECT 应静默不匹配");

    // 对象数组字段配方（SCHEMAFULL）：TYPE array 会拒收对象；
    // FLEXIBLE 派生 items.* 非 FLEXIBLE 会覆盖校验标记，必须 OVERWRITE 补为 FLEXIBLE
    let _ = db.query("REMOVE TABLE IF EXISTS probe_tmp").await;
    let _ = db
        .query(
            "DEFINE TABLE probe_tmp SCHEMAFULL;
             DEFINE FIELD OVERWRITE items ON probe_tmp TYPE array<object> FLEXIBLE;
             DEFINE FIELD OVERWRITE items.* ON probe_tmp TYPE object FLEXIBLE;",
        )
        .await
        .expect("define schemafull");
    let mut res = db
        .query(
            "CREATE probe_tmp CONTENT { items: [{product_id:'x', title:'t', price_cents: 100, qty: 1}] }",
        )
        .await
        .expect("create object array");
    let rows: Vec<Value> = res.take(0).expect("take");
    assert_eq!(rows.len(), 1, "对象数组在 SCHEMAFULL 下应可写入");

    // 清理划痕表
    let _ = db.query("REMOVE TABLE IF EXISTS probe_tmp").await;
}
