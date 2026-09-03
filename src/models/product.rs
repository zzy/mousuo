use serde::{Deserialize, Serialize};

/// 商品
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    /// URL 段（英文 slug）
    pub slug: String,
    /// 商品名
    pub title: String,
    /// Markdown 描述（展示时经 render_md 渲染）
    pub description: String,
    /// 价格（分单位，展示统一走 format_cents）
    pub price_cents: i64,
    /// 库存
    pub stock: i64,
    /// 商品图 URL（试水单图，picsum.photos 占位）
    #[serde(default)]
    pub image: Option<String>,
    /// 状态：active 上架 / off 下架
    pub status: String,
    /// 创建时间（ISO 字符串）
    pub created_at: String,
}
