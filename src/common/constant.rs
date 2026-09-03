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

/// 管理员标记：是（user.is_admin 字段，试水不做 RBAC）
pub const USER_IS_ADMIN: u8 = 1;
