use crate::common::constant::ORDER_STATUS_PENDING;
use crate::db;
use crate::models::order::{Order, OrderItem};
use surrealdb::types::{SurrealValue, Value};

/// 创建订单（状态 pending，items 快照落库）
pub async fn create_order(
    user_id: &str,
    items: Vec<OrderItem>,
    total_cents: i64,
) -> Result<Order, String> {
    let items_json = serde_json::to_value(&items).map_err(|e| e.to_string())?;
    let id = db::new_record_id("order");
    let db = db::get_db();
    let mut res = db
        .query(
            "CREATE order CONTENT { id: $id, user_id: $user, items: $items, total_cents: $total, status: $status, created_at: time::now() }",
        )
        .bind(("id", db::record_id(&id).map_err(|e| e.to_string())?))
        .bind(("user", user_id.to_string()))
        .bind(("items", items_json.into_value()))
        .bind(("total", total_cents))
        .bind(("status", ORDER_STATUS_PENDING))
        .await
        .map_err(|e| e.to_string())?;
    let raw: Vec<Value> = res.take(0).map_err(|e| e.to_string())?;
    raw.iter()
        .filter_map(db::from_value)
        .next()
        .ok_or_else(|| "创建订单失败".to_string())
}

/// 按 id 查订单
pub async fn get_order_by_id(id: &str) -> Result<Option<Order>, String> {
    db::query_one(
        "SELECT * FROM order WHERE id = $id",
        &[("id", db::record_id(id)?)],
    )
    .await
}

/// 按 Stripe Session id 查订单（webhook 关联键）
pub async fn find_order_by_session_id(session_id: &str) -> Result<Option<Order>, String> {
    db::query_one(
        "SELECT * FROM order WHERE stripe_session_id = $sid",
        &[("sid", session_id.to_string().into_value())],
    )
    .await
}

/// 回写 Stripe Session id
pub async fn set_session_id(order_id: &str, session_id: &str) -> Result<(), String> {
    db::get_db()
        .query("UPDATE $id SET stripe_session_id = $sid")
        .bind(("id", db::record_id(order_id)?))
        .bind(("sid", session_id.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 置为已支付（webhook 验签与金额比对通过后调用）
pub async fn mark_paid(order_id: &str) -> Result<(), String> {
    db::get_db()
        .query("UPDATE $id SET status = $status, paid_at = time::now()")
        .bind(("id", db::record_id(order_id)?))
        .bind(("status", "paid".to_string()))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 我的订单分页列表（按创建时间倒序）
pub async fn list_orders(
    user_id: &str,
    page: u64,
    page_size: u64,
) -> Result<Vec<Order>, String> {
    let start = ((page - 1) * page_size) as i64;
    db::query_as(
        "SELECT * FROM order WHERE user_id = $user ORDER BY created_at DESC LIMIT $limit START $start",
        &[
            ("user", user_id.to_string().into_value()),
            ("limit", (page_size as i64).into_value()),
            ("start", start.into_value()),
        ],
    )
    .await
}

/// 我的订单总数
pub async fn count_orders(user_id: &str) -> Result<u64, String> {
    let db = db::get_db();
    let mut res = db
        .query("SELECT count() FROM order WHERE user_id = $user GROUP ALL")
        .bind(("user", user_id.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    let count: Option<u64> = res.take((0, "count")).map_err(|e| e.to_string())?;
    Ok(count.unwrap_or(0))
}

/// 全量订单分页（管理端，按创建时间倒序）
pub async fn list_all_orders(page: u64, page_size: u64) -> Result<Vec<Order>, String> {
    let start = ((page - 1) * page_size) as i64;
    db::query_as(
        "SELECT * FROM order ORDER BY created_at DESC LIMIT $limit START $start",
        &[
            ("limit", (page_size as i64).into_value()),
            ("start", start.into_value()),
        ],
    )
    .await
}

/// 全量订单总数
pub async fn count_all_orders() -> Result<u64, String> {
    let db = db::get_db();
    let mut res = db
        .query("SELECT count() FROM order GROUP ALL")
        .await
        .map_err(|e| e.to_string())?;
    let count: Option<u64> = res.take((0, "count")).map_err(|e| e.to_string())?;
    Ok(count.unwrap_or(0))
}

/// 状态流转（管理端）：条件原子更新，from 不匹配则视为失败（防并发状态跳变）。
/// cancelled 额外写 cancelled_at；合法流转校验见 common/order_state.rs
pub async fn transition_status(order_id: &str, from: &str, to: &str) -> Result<bool, String> {
    let db = db::get_db();
    let sql = if to == crate::common::constant::ORDER_STATUS_CANCELLED {
        "UPDATE $id SET status = $to, cancelled_at = time::now() WHERE status = $from"
    } else {
        "UPDATE $id SET status = $to WHERE status = $from"
    };
    let mut res = db
        .query(sql)
        .bind(("id", db::record_id(order_id)?))
        .bind(("from", from.to_string()))
        .bind(("to", to.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    // 单记录 UPDATE 的返回是对象而非数组，必须用 Vec<Value> 读取
    //（Option<Vec<Value>> 会报 Expected array<any>, got object）
    let updated: Vec<Value> = res.take(0).map_err(|e| e.to_string())?;
    Ok(!updated.is_empty())
}
