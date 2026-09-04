#![allow(non_snake_case)]

use crate::common::constant::{
    ORDER_STATUS_CANCELLED, ORDER_STATUS_COMPLETED, ORDER_STATUS_PAID, ORDER_STATUS_PENDING,
    ORDER_STATUS_SHIPPED,
};
use crate::components::badge::{BadgeVariant, badge};
use crate::components::status_badge::warning_badge;
use crate::i18n::loader;
use topcoat::{
    Result,
    view::{View, attributes, component, view},
};

/// 订单状态文案键
pub fn status_text_key(status: &str) -> &'static str {
    match status {
        ORDER_STATUS_PENDING => "order_status_pending",
        ORDER_STATUS_PAID => "order_status_paid",
        ORDER_STATUS_SHIPPED => "order_status_shipped",
        ORDER_STATUS_COMPLETED => "order_status_completed",
        ORDER_STATUS_CANCELLED => "order_status_cancelled",
        _ => "order_status_pending",
    }
}

/// 订单状态徽章（我的订单页与管理端共用）
///
/// 待支付用警示色（项目自有 warning_badge），其余状态用 registry badge 变体。
#[component]
pub async fn OrderStatusBadge(locale: String, status: String) -> Result<impl View> {
    let text = loader::t(&locale, status_text_key(&status));
    Ok(view! {
        if status == ORDER_STATUS_PAID {
            badge(variant: BadgeVariant::Primary, attrs: attributes! {}, (text))
        } else if status == ORDER_STATUS_CANCELLED {
            badge(variant: BadgeVariant::Destructive, attrs: attributes! {}, (text))
        } else if status == ORDER_STATUS_SHIPPED || status == ORDER_STATUS_COMPLETED {
            badge(variant: BadgeVariant::Secondary, attrs: attributes! {}, (text))
        } else {
            warning_badge(attrs: attributes! {}, (text))
        }
    })
}
