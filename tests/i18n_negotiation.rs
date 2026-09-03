//! i18n 协商与通用辅助函数的纯函数测试（无 DB 依赖，cargo test 直接跑）

use mousuo::common::form::safe_next;
use mousuo::common::format::format_cents;
use mousuo::i18n::loader::{negotiate_language, normalize_locale_path, swap_locale};

#[test]
fn normalize_canonical_supported_path_passes_through() {
    assert_eq!(normalize_locale_path("/zh/products"), None);
    assert_eq!(normalize_locale_path("/en"), None);
    assert_eq!(normalize_locale_path("/zh/products/abc?x=1"), None);
}

#[test]
fn normalize_noncanonical_locale_redirects() {
    assert_eq!(
        normalize_locale_path("/EN/products"),
        Some("/en/products".to_string())
    );
    assert_eq!(
        normalize_locale_path("/en-US/products"),
        Some("/en/products".to_string())
    );
    assert_eq!(
        normalize_locale_path("/zh-hans/products"),
        Some("/zh/products".to_string())
    );
}

#[test]
fn normalize_unsupported_locale_falls_back_to_default() {
    assert_eq!(
        normalize_locale_path("/fr/products"),
        Some("/en/products".to_string())
    );
    assert_eq!(normalize_locale_path("/fr"), Some("/en".to_string()));
}

#[test]
fn normalize_non_locale_paths_pass_through() {
    assert_eq!(normalize_locale_path("/"), None);
    assert_eq!(normalize_locale_path("/assets/foo.css"), None);
    assert_eq!(normalize_locale_path("/_topcoat/assets/x.js"), None);
    assert_eq!(normalize_locale_path("/favicon.svg"), None);
    assert_eq!(normalize_locale_path("/webhook/stripe"), None);
}

#[test]
fn swap_locale_rewrites_first_segment_and_keeps_query() {
    assert_eq!(
        swap_locale("/en/products/wireless-headphones", "zh"),
        "/zh/products/wireless-headphones"
    );
    assert_eq!(
        swap_locale("/zh/products?q=head&page=2", "en"),
        "/en/products?q=head&page=2"
    );
    assert_eq!(swap_locale("/en", "zh"), "/zh");
}

#[test]
fn swap_locale_inserts_segment_for_non_locale_paths() {
    assert_eq!(swap_locale("/products", "zh"), "/zh/products");
    assert_eq!(swap_locale("/", "en"), "/en");
}

// ── Accept-Language 协商（RFC 9110 打分制） ────────────────────────

#[test]
fn negotiate_prefers_highest_q() {
    assert_eq!(negotiate_language("fr, en;q=0.5"), Some("en".to_string()));
    assert_eq!(
        negotiate_language("en;q=0.9, zh;q=0.5"),
        Some("en".to_string())
    );
    assert_eq!(
        negotiate_language("zh;q=0.9, en;q=0.5"),
        Some("zh".to_string())
    );
    assert_eq!(negotiate_language("zh-cn"), Some("zh".to_string()));
}

#[test]
fn negotiate_wildcard_is_not_a_vote_for_default() {
    // 通配只表示“其余语言可接受”，与显式点名同 q 时显式优先
    assert_eq!(
        negotiate_language("*, fr"),
        Some("en".to_string()), // fr 不支持：zh/en 同靠通配，默认 en 胜
    );
    assert_eq!(
        negotiate_language("en;q=0.5, *;q=0.5"),
        Some("en".to_string())
    );
    assert_eq!(
        negotiate_language("*, zh"),
        Some("zh".to_string())
    );
}

#[test]
fn negotiate_ties_break_to_default_locale() {
    assert_eq!(
        negotiate_language("zh;q=1, en;q=1"),
        Some("en".to_string())
    );
}

#[test]
fn negotiate_excludes_q_zero() {
    assert_eq!(negotiate_language("*;q=0, fr"), None);
    assert_eq!(
        negotiate_language("*;q=0, en;q=0.9"),
        Some("en".to_string())
    );
    assert_eq!(negotiate_language(""), None);
}

#[test]
fn negotiate_longest_match_wins_within_language() {
    assert_eq!(
        negotiate_language("fr, en-US;q=0.9, en;q=0.5"),
        Some("en".to_string())
    );
}

#[test]
fn safe_next_accepts_relative_site_paths_only() {
    assert_eq!(safe_next("/zh/orders"), Some("/zh/orders".to_string()));
    assert_eq!(
        safe_next("/zh/products/abc?error=stock"),
        Some("/zh/products/abc?error=stock".to_string())
    );
    assert_eq!(safe_next("//evil.com/x"), None);
    assert_eq!(safe_next("https://evil.com/x"), None);
    assert_eq!(safe_next(""), None);
    assert_eq!(safe_next("/a\r\nb"), None);
}

#[test]
fn format_cents_renders_dollars_without_float() {
    assert_eq!(format_cents(1299), "$12.99");
    assert_eq!(format_cents(599), "$5.99");
    assert_eq!(format_cents(100), "$1.00");
    assert_eq!(format_cents(0), "$0.00");
    assert_eq!(format_cents(-599), "-$5.99");
}
