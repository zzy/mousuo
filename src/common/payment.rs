//! 支付边界（DESIGN 决策 7）：对外仅两个函数，换支付网关只改本文件
//!
//! - `create_checkout`：为订单创建支付会话，返回跳转 URL 与网关会话号
//! - `handle_webhook`：验签 → 事件过滤 → 幂等 → 金额比对 → 置 paid → 原子扣库存
//!
//! 签名说明：DESIGN 定义为 `create_checkout(order) -> url`，实现时补充了
//! success/cancel URL 入参（由调用方从请求上下文拼接，含本地端口）与
//! 会话号返回值（需回写 order.stripe_session_id，不能只返回 URL）。

use crate::common::config;
use crate::common::constant::{ORDER_STATUS_PAID, ORDER_STATUS_PENDING};
use crate::db::{orders, products};
use crate::models::order::Order;
use stripe::{
    CheckoutSession, CheckoutSessionMode, Client, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, CreateCheckoutSessionLineItemsPriceData,
    CreateCheckoutSessionLineItemsPriceDataProductData, Currency, EventObject, EventType, Webhook,
};

/// 创建支付会话：返回 (跳转 URL, 会话号)
pub async fn create_checkout(
    order: &Order,
    success_url: &str,
    cancel_url: &str,
) -> Result<(String, String), String> {
    let cfg = config::config();
    if cfg.stripe_secret_key.is_empty() {
        return Err("支付通道未配置（STRIPE_SECRET_KEY 为空）".to_string());
    }
    let client = Client::new(&cfg.stripe_secret_key);
    // 试水单商品单件；条目快照取首条构建 line_items
    let item = order
        .items
        .first()
        .ok_or_else(|| "订单无条目，无法发起支付".to_string())?;
    let line_item = CreateCheckoutSessionLineItems {
        quantity: Some(item.qty.max(1) as u64),
        price_data: Some(CreateCheckoutSessionLineItemsPriceData {
            currency: Currency::USD,
            unit_amount: Some(item.price_cents),
            product_data: Some(CreateCheckoutSessionLineItemsPriceDataProductData {
                name: item.title.clone(),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut params = CreateCheckoutSession::new();
    params.mode = Some(CheckoutSessionMode::Payment);
    params.success_url = Some(success_url);
    params.cancel_url = Some(cancel_url);
    params.line_items = Some(vec![line_item]);
    let session = CheckoutSession::create(&client, params)
        .await
        .map_err(|e| e.to_string())?;
    let url = session
        .url
        .ok_or_else(|| "Stripe 未返回支付跳转链接".to_string())?;
    Ok((url, session.id.to_string()))
}

/// 处理 Stripe webhook：
/// 1. 验签（失败返回 Err，调用方回 400）
/// 2. 仅处理 checkout.session.completed，其余事件直接 Ok
/// 3. 幂等：按 session id 查订单，已 paid 直接 Ok
/// 4. 金额比对：session.amount_total 必须等于订单 total_cents
/// 5. 置 paid + paid_at
/// 6. 原子扣库存（扣 0 行 = 超卖边界，日志告警，人工处理）
pub async fn handle_webhook(payload: &str, signature: &str) -> Result<(), String> {
    let cfg = config::config();
    if cfg.stripe_webhook_secret.is_empty() {
        return Err("webhook 验签密钥未配置（STRIPE_WEBHOOK_SECRET 为空）".to_string());
    }
    let event = Webhook::construct_event(payload, signature, &cfg.stripe_webhook_secret)
        .map_err(|e| format!("验签失败: {e}"))?;
    if event.type_ != EventType::CheckoutSessionCompleted {
        return Ok(()); // 无关事件直接忽略
    }
    let EventObject::CheckoutSession(session) = event.data.object else {
        return Ok(());
    };
    let session_id = session.id.to_string();
    let Some(order) = orders::find_order_by_session_id(&session_id).await? else {
        return Err(format!("未找到关联订单: {session_id}"));
    };
    // 幂等：重复回调直接成功返回
    if order.status == ORDER_STATUS_PAID {
        return Ok(());
    }
    if order.status != ORDER_STATUS_PENDING {
        return Err(format!(
            "订单状态异常（当前 {}，期望 {}）: {}",
            order.status, ORDER_STATUS_PENDING, order.id
        ));
    }
    // 金额比对：与下单落库金额一致才放行
    let amount = session
        .amount_total
        .ok_or_else(|| format!("回调缺少金额: {session_id}"))?;
    if amount != order.total_cents {
        return Err(format!(
            "金额不符（回调 {}，订单 {}）: {}",
            amount, order.total_cents, order.id
        ));
    }
    orders::mark_paid(&order.id).await?;
    // 原子扣库存（单语句 WHERE stock > 0 防超卖）
    for item in &order.items {
        match products::decrement_stock(&item.product_id, item.qty).await {
            Ok(true) => {}
            Ok(false) => {
                eprintln!(
                    "超卖告警: 订单 {} 已支付但商品 {} 库存不足",
                    order.id, item.product_id
                );
            }
            Err(e) => {
                eprintln!(
                    "库存扣减异常: 订单 {} 商品 {}: {e}",
                    order.id, item.product_id
                );
            }
        }
    }
    Ok(())
}
