use crate::db;

use crate::models::arc::Arc;
use surrealdb::types::{SurrealValue, Value};

fn build_where(
    arc_type: Option<&str>,
    search: Option<&str>,
) -> (String, Vec<(&'static str, Value)>) {
    let mut clauses = Vec::new();
    let mut params: Vec<(&'static str, Value)> = Vec::new();
    if let Some(pt) = arc_type {
        clauses.push("arc_type = $arc_type");
        params.push(("arc_type", pt.to_string().into_value()));
    }
    if let Some(q) = search {
        if !q.is_empty() {
            clauses.push("(title ~ $search OR body ~ $search OR topics ~ $search)");
            params.push(("search", q.to_string().into_value()));
        }
    }
    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };
    (where_clause, params)
}

/// 分页查询 arc
pub async fn get_arcs(
    arc_type: Option<&str>,
    search: Option<&str>,
    page: u64,
    page_size: u64,
) -> Result<Vec<Arc>, String> {
    let start = ((page - 1) * page_size) as i64;
    let (where_clause, mut params) = build_where(arc_type, search);
    params.push(("limit", (page_size as i64).into_value()));
    params.push(("start", start.into_value()));
    let sql = format!(
        "SELECT *, id.id() AS id FROM arc{where_clause} ORDER BY created_at DESC LIMIT $limit START $start"
    );
    db::query_as(&sql, &params).await
}

/// 统计 arc 总数
pub async fn count_arcs(arc_type: Option<&str>, search: Option<&str>) -> Result<u64, String> {
    let (where_clause, params) = build_where(arc_type, search);
    let sql = format!("SELECT count() FROM arc{where_clause} GROUP ALL");
    let db = db::get_db();
    let mut query = db.query(&sql);
    for (k, v) in &params {
        query = query.bind((*k, v.clone()));
    }
    let mut res = query.await.map_err(|e| e.to_string())?;
    let count: Option<u64> = res.take((0, "count")).map_err(|e| e.to_string())?;
    Ok(count.unwrap_or(0))
}

// ── 首页数据（请求级缓存，Demo 模式：memoize + 公开包装函数）──

/// 缓存的查询（仅 cx + Copy 参数，同请求内多次调用只查一次 DB）
#[topcoat::context::memoize(as_ref)]
async fn query_home(cx: &topcoat::context::Cx, limit: u64) -> Result<Vec<Arc>, String> {
    db::query_as(
        "SELECT *, id.id() AS id FROM arc ORDER BY created_at DESC LIMIT $limit",
        &[("limit", (limit as i64).into_value())],
    )
    .await
}

/// 首页数据公开接口（委托 memoize 的 query_home）
pub async fn get_home_arcs(
    cx: &topcoat::context::Cx,
    limit: u64,
) -> Result<(Vec<Arc>, Vec<Arc>, Vec<Arc>), String> {
    let all = query_home(cx, limit).await.map_err(|e| e.to_string())?;
    let mut videos = Vec::new();
    let mut articles = Vec::new();
    let mut photos = Vec::new();
    for e in all.iter() {
        match e.arc_type.as_str() {
            "video" if videos.len() < 3 => videos.push(e.clone()),
            "article" if articles.len() < 3 => articles.push(e.clone()),
            "photo" if photos.len() < 3 => photos.push(e.clone()),
            _ => {}
        }
        if videos.len() >= 3 && articles.len() >= 3 && photos.len() >= 3 {
            break;
        }
    }
    Ok((videos, articles, photos))
}

// ── 其余查询 ────────────────────────────────────────────────────

pub async fn get_arc_by_id(id: &str) -> Result<Option<Arc>, String> {
    db::query_one(
        "SELECT *, id.id() AS id FROM arc WHERE id = type::record('arc', $id)",
        &[("id", id.to_string().into_value())],
    )
    .await
}

/// 按 slug 查详情（memoize：布局标题与详情页同请求共享，零重复查询）
#[topcoat::context::memoize(as_ref)]
async fn arc_by_slug(cx: &topcoat::context::Cx, slug: &str) -> Result<Option<Arc>, String> {
    db::query_one(
        "SELECT *, id.id() AS id FROM arc WHERE slug = $slug",
        &[("slug", slug.to_string().into_value())],
    )
    .await
}

/// 公开接口（委托 memoize 的 arc_by_slug）
pub async fn get_arc_by_slug(cx: &topcoat::context::Cx, slug: &str) -> Result<Option<Arc>, String> {
    arc_by_slug(cx, slug)
        .await
        .map(|o| o.clone())
        .map_err(|e| e.to_string())
}

/// slug 合法性：小写字母/数字/连字符，不以连字符开头或结尾，长度 2..=64
pub fn valid_slug(slug: &str) -> bool {
    let len = slug.len();
    if !(2..=64).contains(&len) {
        return false;
    }
    let first = slug.chars().next().unwrap_or_default();
    let last = slug.chars().last().unwrap_or_default();
    if first == '-' || last == '-' {
        return false;
    }
    slug.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// slug 是否已被占用
pub async fn slug_exists(slug: &str) -> Result<bool, String> {
    let found = db::query_one::<serde_json::Value>(
        "SELECT id FROM arc WHERE slug = $slug",
        &[("slug", slug.to_string().into_value())],
    )
    .await?
    .is_some();
    Ok(found)
}

/// 标题 → slug 候选：小写、非字母数字转连字符、去首尾连字符（中文标题等会得到空串）
pub fn slugify_title(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut last_dash = false;
    for ch in title.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// 唯一 slug：候选非法或已被占用时，追加随机后缀（连字符连接）保证可用
pub async fn unique_slug(base: &str) -> String {
    let suffix = || crate::common::rand::random_hex().chars().take(6).collect::<String>();
    let mut candidate = if valid_slug(base) {
        base.to_string()
    } else {
        suffix()
    };
    loop {
        if !slug_exists(&candidate).await.unwrap_or(false) {
            return candidate;
        }
        candidate = format!("{candidate}-{}", suffix());
    }
}

pub async fn get_arcs_by_author(
    author_name: &str,
    page: u64,
    page_size: u64,
) -> Result<Vec<Arc>, String> {
    let start = ((page - 1) * page_size) as i64;
    db::query_as(
        "SELECT *, id.id() AS id FROM arc WHERE author_name = $author_name ORDER BY created_at DESC LIMIT $limit START $start",
        &[
            ("author_name", author_name.to_string().into_value()),
            ("limit", (page_size as i64).into_value()),
            ("start", start.into_value()),
        ],
    )
    .await
}

pub async fn count_arcs_by_author(author_name: &str) -> Result<u64, String> {
    let db = db::get_db();
    let mut res = db
        .query("SELECT count() FROM arc WHERE author_name = $author_name GROUP ALL")
        .bind(("author_name", author_name.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    let count: Option<u64> = res.take((0, "count")).map_err(|e| e.to_string())?;
    Ok(count.unwrap_or(0))
}

pub async fn create_arc(
    slug: &str,
    title: &str,
    arc_type: &str,
    media_url: &str,
    body: Option<&str>,
    topics: Option<&str>,
    author_name: Option<&str>,
    thumbnail: Option<&str>,
) -> Result<(), String> {
    let id = db::new_record_key();
    let db = db::get_db();
    db.query(
        "CREATE arc CONTENT { id: type::record('arc', $id), slug: $slug, title: $title, arc_type: $arc_type, media_url: $media_url, body: $body, topics: $topics, author_name: $author_name, thumbnail: $thumbnail, created_at: time::now(), view_count: 0 }",
    )
    .bind(("id", id))
    .bind(("slug", slug.to_string()))
    .bind(("title", title.to_string()))
    .bind(("arc_type", arc_type.to_string()))
    .bind(("media_url", media_url.to_string()))
    .bind(("body", body.unwrap_or("")))
    .bind(("topics", topics.unwrap_or("")))
    .bind(("author_name", author_name.unwrap_or("")))
    .bind(("thumbnail", thumbnail.unwrap_or("")))
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除 arc 并清理其独占媒体（media_url 与 body 内嵌 /media/ 引用；
/// 被其他 arc 复用的文件保留）
pub async fn delete_arc(id: &str) -> Result<(), String> {
    // 删除前收集媒体引用
    let media_refs = match get_arc_by_id(id).await? {
        Some(arc) => {
            let mut refs = Vec::new();
            if let Some(media) = arc.media_url.as_deref().filter(|s| !s.is_empty()) {
                refs.push(media.to_string());
            }
            if let Some(body) = arc.body.as_deref() {
                refs.extend(
                    crate::common::media::extract_media_urls(body)
                        .into_iter()
                        .map(str::to_string),
                );
            }
            refs
        }
        None => return Err("内容不存在".to_string()),
    };
    db::get_db()
        .query("DELETE arc WHERE id = type::record('arc', $id)")
        .bind(("id", id.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    // 仅清理无其他 arc 引用的媒体（防误删复用文件）
    for url in media_refs {
        let mut res = db::get_db()
            .query(
                "SELECT id FROM arc WHERE id != type::record('arc', $id) AND (media_url = $url OR body CONTAINS $url)",
            )
            .bind(("id", id.to_string()))
            .bind(("url", url.clone()))
            .await
            .map_err(|e| e.to_string())?;
        let rows: Vec<Value> = res.take(0).map_err(|e| e.to_string())?;
        if rows.is_empty() {
            if let Err(e) = crate::common::media::remove_upload(&url).await {
                eprintln!("媒体清理失败 {url}: {e}");
            }
        }
    }
    Ok(())
}

// ── 种子数据 ──────────────────────────────────────────────────────────────

/// 演示条目种子
struct SeedArc {
    slug: &'static str,
    title: &'static str,
    arc_type: &'static str,
    media_url: Option<&'static str>,
    body: &'static str,
    topics: &'static str,
}

/// 9 个演示条目（3 图文 / 3 视频 / 3 照片；缩略图 picsum 占位，视频为公开样例片源）
const SEED_ARCS: [SeedArc; 9] = [
    SeedArc {
        slug: "morning-at-the-harbor",
        title: "Morning at the Harbor",
        arc_type: "article",
        media_url: None,
        body: "## 晨光初照\n\n清晨的港口总是醒得最早。\n\n船桅在薄雾里晃动，海鸥掠过水面，鱼市的第一批摊主已经就位。\n\n![港口晨景](https://picsum.photos/seed/harbor-detail/900/500)\n\n> 城市的一天，从水面上的第一缕光开始。",
        topics: "life, city",
    },
    SeedArc {
        slug: "the-old-bookstore",
        title: "The Old Bookstore",
        arc_type: "article",
        media_url: None,
        body: "## 巷子里的旧书店\n\n推开木门，纸墨与旧木地板的气息扑面而来。\n\n- 顶到天花板的书架\n- 老板手写的推荐便签\n- 一只永远睡在窗台上的猫\n\n![旧书店一角](https://picsum.photos/seed/bookstore-detail/900/500)\n\n> 有些地方，时间走得更慢。",
        topics: "culture, city",
    },
    SeedArc {
        slug: "a-walk-in-autumn",
        title: "A Walk in Autumn",
        arc_type: "article",
        media_url: None,
        body: "## 秋日散步\n\n银杏落了满街，踩上去沙沙作响。\n\n风把云推得很高，天蓝得发脆。走慢一点，就能听见叶子落地的声音。\n\n![秋日街景](https://picsum.photos/seed/autumn-detail/900/500)",
        topics: "life, nature",
    },
    SeedArc {
        slug: "big-buck-bunny",
        title: "Big Buck Bunny",
        arc_type: "video",
        media_url: Some("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"),
        body: "经典开源动画短片《Big Buck Bunny》，用于演示视频播放与 HLS 转码流程。",
        topics: "animation, video",
    },
    SeedArc {
        slug: "sintel-trailer",
        title: "Sintel Trailer",
        arc_type: "video",
        media_url: Some("https://media.w3.org/2010/05/sintel/trailer.mp4"),
        body: "Blender 开源电影《Sintel》预告片，画面细腻，适合演示高清播放。",
        topics: "film, video",
    },
    SeedArc {
        slug: "elephants-dream",
        title: "Elephants Dream",
        arc_type: "video",
        media_url: Some("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ElephantsDream.mp4"),
        body: "首部完全用开源软件制作的电影《Elephants Dream》，超现实主义风格。",
        topics: "animation, video",
    },
    SeedArc {
        slug: "golden-hour",
        title: "Golden Hour",
        arc_type: "photo",
        media_url: Some("https://picsum.photos/seed/golden_hour/1200/800"),
        body: "日落前的一小时，光线最温柔。",
        topics: "photo, nature",
    },
    SeedArc {
        slug: "rainy-window",
        title: "Rainy Window",
        arc_type: "photo",
        media_url: Some("https://picsum.photos/seed/rainy_window/1200/800"),
        body: "雨天的窗，是城市最好的滤镜。",
        topics: "photo, city",
    },
    SeedArc {
        slug: "mountain-mist",
        title: "Mountain Mist",
        arc_type: "photo",
        media_url: Some("https://picsum.photos/seed/mountain_mist/1200/800"),
        body: "山间的雾，来去都悄无声息。",
        topics: "photo, nature",
    },
];

/// 启动种子：arc 表为空时插入演示条目
pub async fn seed_arcs() -> Result<(), String> {
    let db = db::get_db();
    let mut res = db
        .query("SELECT count() FROM arc GROUP ALL")
        .await
        .map_err(|e| e.to_string())?;
    let count: Option<u64> = res.take((0, "count")).map_err(|e| e.to_string())?;
    if count.unwrap_or(0) > 0 {
        return Ok(());
    }
    for item in SEED_ARCS {
        let thumbnail = format!("https://picsum.photos/seed/{}/800/600", item.slug);
        create_arc(
            item.slug,
            item.title,
            item.arc_type,
            item.media_url.unwrap_or(""),
            Some(item.body),
            Some(item.topics),
            Some("Demo"),
            Some(&thumbnail),
        )
        .await?;
    }
    Ok(())
}
