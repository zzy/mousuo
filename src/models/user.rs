use serde::{Deserialize, Serialize};

/// 用户
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    /// Argon2id PHC 哈希，禁止序列化到客户端
    #[serde(skip_serializing)]
    pub cred: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub introduction: String,
    /// 账户状态：0=待激活，1=正常，2=已封禁
    #[serde(default)]
    pub status: u8,
    /// 管理员标记：1=管理员（后台任免设置，首个管理员手工改库）
    #[serde(default)]
    pub is_admin: u8,
}
