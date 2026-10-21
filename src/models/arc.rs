use serde::{Deserialize, Serialize};

/// 内容类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ArcType {
    Article,
    Video,
    Photo,
}

impl ArcType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Article => "article",
            Self::Video => "video",
            Self::Photo => "photo",
        }
    }
}

/// Arc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arc {
    pub id: String,
    pub title: String,
    /// URL 段（英文 slug）
    pub slug: String,
    #[serde(rename = "arc_type")]
    pub arc_type: ArcType,
    pub body: Option<String>,
    pub summary: Option<String>,
    pub thumbnail: Option<String>,
    #[serde(rename = "media_url")]
    pub media_url: Option<String>,
    #[serde(rename = "author_name")]
    pub author_name: Option<String>,
    pub topics: Option<String>,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "view_count")]
    pub view_count: u64,
}
