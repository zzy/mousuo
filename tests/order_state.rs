//! 订单状态机纯函数测试（管理端流转合法性）

use mousuo::common::constant::{
    ORDER_STATUS_CANCELLED, ORDER_STATUS_COMPLETED, ORDER_STATUS_PAID, ORDER_STATUS_PENDING,
    ORDER_STATUS_SHIPPED,
};
use mousuo::common::order_state::{allowed_transition, next_actions};

#[test]
fn allows_legal_transitions() {
    assert!(allowed_transition(ORDER_STATUS_PENDING, ORDER_STATUS_CANCELLED));
    assert!(allowed_transition(ORDER_STATUS_PAID, ORDER_STATUS_SHIPPED));
    assert!(allowed_transition(ORDER_STATUS_PAID, ORDER_STATUS_CANCELLED));
    assert!(allowed_transition(ORDER_STATUS_SHIPPED, ORDER_STATUS_COMPLETED));
}

#[test]
fn rejects_illegal_transitions() {
    // 跳过中间态、反向、自旋、终态再流转、未知状态
    for (from, to) in [
        (ORDER_STATUS_PENDING, ORDER_STATUS_SHIPPED),
        (ORDER_STATUS_PENDING, ORDER_STATUS_COMPLETED),
        (ORDER_STATUS_PENDING, ORDER_STATUS_PAID), // webhook 专属，管理端不允许
        (ORDER_STATUS_PAID, ORDER_STATUS_COMPLETED),
        (ORDER_STATUS_PAID, ORDER_STATUS_PENDING),
        (ORDER_STATUS_SHIPPED, ORDER_STATUS_PAID),
        (ORDER_STATUS_SHIPPED, ORDER_STATUS_CANCELLED),
        (ORDER_STATUS_COMPLETED, ORDER_STATUS_SHIPPED),
        (ORDER_STATUS_COMPLETED, ORDER_STATUS_CANCELLED),
        (ORDER_STATUS_CANCELLED, ORDER_STATUS_PENDING),
        (ORDER_STATUS_CANCELLED, ORDER_STATUS_PAID),
        (ORDER_STATUS_PAID, ORDER_STATUS_PAID),
        ("unknown", ORDER_STATUS_SHIPPED),
        (ORDER_STATUS_PAID, "unknown"),
    ] {
        assert!(
            !allowed_transition(from, to),
            "非法流转不应允许: {from} -> {to}"
        );
    }
}

#[test]
fn exposes_expected_actions_per_status() {
    assert_eq!(next_actions(ORDER_STATUS_PENDING), &["admin_cancel_order"]);
    assert_eq!(
        next_actions(ORDER_STATUS_PAID),
        &["admin_mark_shipped", "admin_cancel_order"]
    );
    assert_eq!(next_actions(ORDER_STATUS_SHIPPED), &["admin_mark_completed"]);
    assert!(next_actions(ORDER_STATUS_COMPLETED).is_empty());
    assert!(next_actions(ORDER_STATUS_CANCELLED).is_empty());
    assert!(next_actions("unknown").is_empty());
}
