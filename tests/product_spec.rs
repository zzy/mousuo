//! 商品 slug 校验纯函数测试（管理端创建/编辑共用）

use mousuo::db::products::valid_slug;

#[test]
fn accepts_valid_slugs() {
    for slug in ["a1", "my-product", "abc123", "x-y-z", "zh", "n9-0k"] {
        assert!(valid_slug(slug), "应合法: {slug}");
    }
}

#[test]
fn rejects_invalid_slugs() {
    for slug in [
        "",
        "a",
        "A",
        "My-Product",
        "中文",
        "a b",
        "-abc",
        "abc-",
        "a--b--", // 以连字符结尾
        "abc_def",
        "a/b",
        "a.b",
    ] {
        assert!(!valid_slug(slug), "应非法: {slug:?}");
    }
}

#[test]
fn enforces_length_bounds() {
    assert!(!valid_slug(&"a".repeat(1)));
    assert!(valid_slug(&"a".repeat(64)));
    assert!(!valid_slug(&"a".repeat(65)));
}
