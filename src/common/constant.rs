/// 账户状态：待激活
pub const USER_STATUS_PENDING: u8 = 0;
/// 账户状态：正常
pub const USER_STATUS_ACTIVE: u8 = 1;
/// 账户状态：已封禁
pub const USER_STATUS_BANNED: u8 = 2;

/// 验证码有效期（秒）
pub const CAPTCHA_EXPIRY: u64 = 300;

/// 密码重置链接有效期（秒）
pub const PASSWORD_RESET_EXPIRY: u64 = 3600;

/// 商品状态：上架
pub const PRODUCT_STATUS_ACTIVE: &str = "active";
/// 商品状态：下架（P3 管理端）
pub const PRODUCT_STATUS_OFF: &str = "off";
/// 低库存阈值：库存 ≤ 该值显示低库存徽章
pub const PRODUCT_LOW_STOCK_THRESHOLD: i64 = 5;

/// 订单状态：待支付
pub const ORDER_STATUS_PENDING: &str = "pending";
/// 订单状态：已支付
pub const ORDER_STATUS_PAID: &str = "paid";
/// 订单状态：已发货（P3 管理端流转）
pub const ORDER_STATUS_SHIPPED: &str = "shipped";
/// 订单状态：已完成（P3 管理端流转）
pub const ORDER_STATUS_COMPLETED: &str = "completed";
/// 订单状态：已取消（P3 管理端流转）
pub const ORDER_STATUS_CANCELLED: &str = "cancelled";

/// 管理员标记：是（user.is_admin 字段）
pub const USER_IS_ADMIN: u8 = 1;
/// 错误码 → i18n 键（业务错误码映射，各项目自持）
pub fn error_i18n_key(err: &str) -> Option<&'static str> {
    Some(match err {
        "captcha" => "captcha_invalid",
        "incorrect" => "sign_in_incorrect",
        "not_activation" => "sign_in_not_activation",
        "banned" => "sign_in_banned",
        "security" => "sign_in_security_problem",
        "password_weak" => "register_password_weak",
        "password_mismatch" => "register_password_mismatch",
        "exist" => "register_exist",
        "payment" => "checkout_failed",
        "invalid" => "checkout_invalid",
        "stock" => "product_out_of_stock",
        "slug_invalid" => "admin_form_slug_invalid",
        "slug_exists" => "admin_form_slug_exists",
        "title_invalid" => "admin_form_title_invalid",
        "description_invalid" => "admin_form_description_invalid",
        "price_invalid" => "admin_form_price_invalid",
        "stock_invalid" => "admin_form_stock_invalid",
        "create_failed" => "admin_form_create_failed",
        "update_failed" => "admin_form_update_failed",
        "upload_empty" => "admin_upload_empty",
        "upload_too_large" => "admin_upload_too_large",
        "upload_type" => "admin_upload_type_invalid",
        "upload_failed" => "admin_upload_failed",
        _ => return None,
    })
}
