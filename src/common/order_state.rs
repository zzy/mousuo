//! 订单状态机 — 管理端流转合法性（纯函数，供测试与页面共用）
//!
//! 状态机（DESIGN.md 三）：
//!   pending --(webhook)--> paid --(管理端)--> shipped --(管理端)--> completed
//!      |                      |
//!      +---(管理端取消)--------+----(管理端取消)--> cancelled

use crate::common::constant::{
    ORDER_STATUS_CANCELLED, ORDER_STATUS_COMPLETED, ORDER_STATUS_PAID, ORDER_STATUS_PENDING,
    ORDER_STATUS_SHIPPED,
};

/// from → to 是否合法
pub fn allowed_transition(from: &str, to: &str) -> bool {
    match (from, to) {
        (ORDER_STATUS_PAID, ORDER_STATUS_SHIPPED) => true,
        (ORDER_STATUS_SHIPPED, ORDER_STATUS_COMPLETED) => true,
        (ORDER_STATUS_PENDING, ORDER_STATUS_CANCELLED) => true,
        (ORDER_STATUS_PAID, ORDER_STATUS_CANCELLED) => true,
        _ => false,
    }
}

/// 当前状态下管理端可执行的动作（i18n 键）
pub fn next_actions(status: &str) -> &'static [&'static str] {
    match status {
        ORDER_STATUS_PENDING => &["admin_cancel_order"],
        ORDER_STATUS_PAID => &["admin_mark_shipped", "admin_cancel_order"],
        ORDER_STATUS_SHIPPED => &["admin_mark_completed"],
        _ => &[],
    }
}
