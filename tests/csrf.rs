//! CSRF token 纯函数测试（恒定时间比较与随机生成）

use mousuo::common::session::{ct_eq, generate_csrf_token};

#[test]
fn ct_eq_compares_identical_strings() {
    assert!(ct_eq("abc123", "abc123"));
    assert!(ct_eq("", ""));
}

#[test]
fn ct_eq_rejects_different_strings() {
    assert!(!ct_eq("abc123", "abc124"));
    assert!(!ct_eq("abc123", "abc12"));
    assert!(!ct_eq("abc123", ""));
    assert!(!ct_eq("", "a"));
    // 等长但逐字节不同
    assert!(!ct_eq("aaaa", "bbbb"));
}

#[test]
fn generate_csrf_token_is_64_hex_chars() {
    let token = generate_csrf_token();
    assert_eq!(token.len(), 64, "32 字节 → 64 位 hex");
    assert!(
        token.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
        "必须为小写 hex"
    );
}

#[test]
fn generate_csrf_token_is_unique() {
    let a = generate_csrf_token();
    let b = generate_csrf_token();
    assert_ne!(a, b, "32 字节随机空间碰撞概率可忽略");
}
