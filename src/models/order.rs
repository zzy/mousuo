use serde::{Deserialize, Serialize};

/// 订单条目（下单时的商品名与价格快照）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: String,
    pub title: String,
    pub price_cents: i64,
    pub qty: i64,
}

/// 订单（状态机见 DESIGN.md：pending → paid → shipped → completed / cancelled）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    /// 下单用户（用户名，user 表 username 字段）
    pub user_id: String,
    /// 条目快照数组
    pub items: Vec<OrderItem>,
    /// 总额（分单位，下单落库；回调金额比对以此为准）
    pub total_cents: i64,
    /// 状态：pending / paid / shipped / completed / cancelled
    pub status: String,
    /// Stripe Checkout Session 关联键（不靠 metadata）
    #[serde(default)]
    pub stripe_session_id: Option<String>,
    #[serde(default)]
    pub paid_at: Option<String>,
    #[serde(default)]
    pub cancelled_at: Option<String>,
    pub created_at: String,
}
