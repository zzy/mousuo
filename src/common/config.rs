use std::sync::LazyLock;

/// 应用配置（从环境变量加载）
pub struct Config {
    /// 站点域名（不含协议，如 localhost）
    pub domain: String,
    /// 分页每页条数
    pub page_size: i64,
    /// SurrealDB 连接地址
    pub db_url: String,
    /// SurrealDB 命名空间
    pub db_ns: String,
    /// SurrealDB 数据库名
    pub db_name: String,
    /// SurrealDB 用户名
    pub db_user: String,
    /// SurrealDB 密码
    pub db_pass: String,
    /// SMTP 服务器地址
    pub email_smtp: String,
    /// 邮件发件地址
    pub email_from: String,
    /// SMTP 认证用户名
    pub email_username: String,
    /// SMTP 认证密码
    pub email_password: String,
    /// Stripe 密钥（test/live）；未配置时为空串，支付功能报错但不阻塞启动
    pub stripe_secret_key: String,
    /// Stripe webhook 验签密钥；未配置时为空串，回调验签必然失败
    pub stripe_webhook_secret: String,
}

static CFG: LazyLock<Config> = LazyLock::new(|| {
    dotenvy::dotenv().ok();
    Config {
        domain: env("DOMAIN"),
        page_size: parse("PAGE_SIZE"),
        db_url: env("DB_URL"),
        db_ns: env("DB_NS"),
        db_name: env("DB_NAME"),
        db_user: env("DB_USER"),
        db_pass: env("DB_PASS"),
        email_smtp: env("EMAIL_SMTP"),
        email_from: env("EMAIL_FROM"),
        email_username: env("EMAIL_USERNAME"),
        email_password: env("EMAIL_PASSWORD"),
        stripe_secret_key: env_or_default("STRIPE_SECRET_KEY"),
        stripe_webhook_secret: env_or_default("STRIPE_WEBHOOK_SECRET"),
    }
});

pub fn config() -> &'static Config {
    &CFG
}

fn env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set"))
}

/// 可选配置：缺失时为空串（支付功能在使用点报错，不阻塞服务启动）
fn env_or_default(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}

fn parse<T: std::str::FromStr>(key: &str) -> T
where
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    std::env::var(key)
        .unwrap_or_else(|_| panic!("{key} must be set"))
        .parse()
        .unwrap_or_else(|e| panic!("{key} must be a valid integer: {e}"))
}
