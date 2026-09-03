use crate::common::constant::PRODUCT_STATUS_ACTIVE;
use crate::db;
use crate::models::product::Product;
use surrealdb::types::{SurrealValue, Value};

/// 构造商品查询条件（仅上架 + 可选标题模糊搜索）
fn build_where(search: Option<&str>) -> (String, Vec<(&'static str, Value)>) {
    let mut clauses = vec!["status = $status"];
    let mut params: Vec<(&'static str, Value)> =
        vec![("status", PRODUCT_STATUS_ACTIVE.into_value())];
    if let Some(q) = search.filter(|s| !s.is_empty()) {
        // 该库版本不支持 ~ 模糊匹配，采用小写化后子串匹配
        clauses.push("string::lowercase(title) CONTAINS $search");
        params.push(("search", q.to_lowercase().into_value()));
    }
    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };
    (where_clause, params)
}

/// 分页列表（仅上架商品，按创建时间倒序）
pub async fn list_products(
    search: Option<&str>,
    page: u64,
    page_size: u64,
) -> Result<Vec<Product>, String> {
    let start = ((page - 1) * page_size) as i64;
    let (where_clause, mut params) = build_where(search);
    params.push(("limit", (page_size as i64).into_value()));
    params.push(("start", start.into_value()));
    let sql = format!(
        "SELECT * FROM product{where_clause} ORDER BY created_at DESC, id LIMIT $limit START $start"
    );
    db::query_as(&sql, &params).await
}

/// 统计上架商品总数
pub async fn count_products(search: Option<&str>) -> Result<u64, String> {
    let (where_clause, params) = build_where(search);
    let sql = format!("SELECT count() FROM product{where_clause} GROUP ALL");
    let db = db::get_db();
    let mut query = db.query(&sql);
    for (k, v) in &params {
        query = query.bind((*k, v.clone()));
    }
    let mut res = query.await.map_err(|e| e.to_string())?;
    let count: Option<u64> = res.take((0, "count")).map_err(|e| e.to_string())?;
    Ok(count.unwrap_or(0))
}

/// 按 slug 查详情（仅上架）
pub async fn get_product_by_slug(slug: &str) -> Result<Option<Product>, String> {
    db::query_one(
        "SELECT * FROM product WHERE slug = $slug AND status = $status",
        &[
            ("slug", slug.to_string().into_value()),
            ("status", PRODUCT_STATUS_ACTIVE.into_value()),
        ],
    )
    .await
}

/// 按 slug 查商品（含下架，管理端唯一性校验用）
pub async fn get_product_by_slug_any(slug: &str) -> Result<Option<Product>, String> {
    db::query_one(
        "SELECT * FROM product WHERE slug = $slug",
        &[("slug", slug.to_string().into_value())],
    )
    .await
}

/// 全量商品分页（管理端，含下架，按创建时间倒序）
pub async fn list_all_products(page: u64, page_size: u64) -> Result<Vec<Product>, String> {
    let start = ((page - 1) * page_size) as i64;
    db::query_as(
        "SELECT * FROM product ORDER BY created_at DESC, id LIMIT $limit START $start",
        &[
            ("limit", (page_size as i64).into_value()),
            ("start", start.into_value()),
        ],
    )
    .await
}

/// 全量商品总数
pub async fn count_all_products() -> Result<u64, String> {
    let db = db::get_db();
    let mut res = db
        .query("SELECT count() FROM product GROUP ALL")
        .await
        .map_err(|e| e.to_string())?;
    let count: Option<u64> = res.take((0, "count")).map_err(|e| e.to_string())?;
    Ok(count.unwrap_or(0))
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

/// slug 是否已被占用（含下架商品，创建/编辑唯一性校验）
pub async fn slug_exists(slug: &str) -> Result<bool, String> {
    let found = db::query_one::<serde_json::Value>(
        "SELECT id FROM product WHERE slug = $slug",
        &[("slug", slug.to_string().into_value())],
    )
    .await?
    .is_some();
    Ok(found)
}

/// 创建商品（管理端；slug 唯一性由调用方先校验）
pub async fn create_product(
    slug: &str,
    title: &str,
    description: &str,
    price_cents: i64,
    stock: i64,
    image: Option<&str>,
) -> Result<Product, String> {
    let id = db::new_record_id("product");
    let db = db::get_db();
    let mut res = db
        .query(
            "CREATE product CONTENT { id: $id, slug: $slug, title: $title, description: $description, price_cents: $price, stock: $stock, image: $image, status: $status, created_at: time::now() }",
        )
        .bind(("id", db::record_id(&id).map_err(|e| e.to_string())?))
        .bind(("slug", slug.to_string()))
        .bind(("title", title.to_string()))
        .bind(("description", description.to_string()))
        .bind(("price", price_cents))
        .bind(("stock", stock))
        .bind(("image", image.filter(|s| !s.is_empty()).map(str::to_string).into_value()))
        .bind(("status", PRODUCT_STATUS_ACTIVE))
        .await
        .map_err(|e| e.to_string())?;
    let raw: Vec<Value> = res.take(0).map_err(|e| e.to_string())?;
    raw.iter()
        .filter_map(db::from_value)
        .next()
        .ok_or_else(|| "创建商品失败".to_string())
}

/// 更新商品基本信息（管理端；不改 status）
pub async fn update_product(
    product_id: &str,
    slug: &str,
    title: &str,
    description: &str,
    price_cents: i64,
    stock: i64,
    image: Option<&str>,
) -> Result<(), String> {
    db::get_db()
        .query(
            "UPDATE $id SET slug = $slug, title = $title, description = $description, price_cents = $price, stock = $stock, image = $image",
        )
        .bind(("id", db::record_id(product_id)?))
        .bind(("slug", slug.to_string()))
        .bind(("title", title.to_string()))
        .bind(("description", description.to_string()))
        .bind(("price", price_cents))
        .bind(("stock", stock))
        .bind(("image", image.filter(|s| !s.is_empty()).map(str::to_string).into_value()))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 上架/下架（管理端）
pub async fn set_product_status(product_id: &str, status: &str) -> Result<(), String> {
    db::get_db()
        .query("UPDATE $id SET status = $status")
        .bind(("id", db::record_id(product_id)?))
        .bind(("status", status.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除商品（管理端）：删除 DB 记录，并清理其独占媒体文件
/// （image 与描述内嵌 /media/ 引用；被其他商品复用的文件保留）
pub async fn delete_product(product_id: &str) -> Result<(), String> {
    // 删除前收集媒体引用（image + description 内嵌）
    let media_refs = match get_product_by_id(product_id).await? {
        Some(product) => {
            let mut refs = Vec::new();
            if let Some(image) = product.image.as_deref().filter(|s| !s.is_empty()) {
                refs.push(image.to_string());
            }
            refs.extend(
                crate::common::media::extract_media_urls(&product.description)
                    .into_iter()
                    .map(str::to_string),
            );
            refs
        }
        None => return Err("商品不存在".to_string()),
    };
    db::get_db()
        .query("DELETE product WHERE id = $id")
        .bind(("id", db::record_id(product_id)?))
        .await
        .map_err(|e| e.to_string())?;
    // 仅清理无其他商品引用的媒体（误删复用在用文件）
    for url in media_refs {
        let mut res = db::get_db()
            .query(
                "SELECT id FROM product WHERE id != $id AND (image = $url OR description CONTAINS $url)",
            )
            .bind(("id", db::record_id(product_id)?))
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

/// 按 id 查商品（含下架，checkout 校验与订单回链用）
pub async fn get_product_by_id(id: &str) -> Result<Option<Product>, String> {
    db::query_one(
        "SELECT * FROM product WHERE id = $id",
        &[("id", db::record_id(id)?)],
    )
    .await
}

/// 原子扣减库存（单语句，防超卖）：stock >= qty 才扣，否则返回未扣减
pub async fn decrement_stock(product_id: &str, qty: i64) -> Result<bool, String> {
    let db = db::get_db();
    let mut res = db
        .query("UPDATE $id SET stock = stock - $qty WHERE stock >= $qty")
        .bind(("id", db::record_id(product_id)?))
        .bind(("qty", qty))
        .await
        .map_err(|e| e.to_string())?;
    // 单记录 UPDATE 的返回是对象而非数组，必须用 Vec<Value> 读取
    //（Option<Vec<Value>> 会报 Expected array<any>, got object）
    let updated: Vec<Value> = res.take(0).map_err(|e| e.to_string())?;
    Ok(!updated.is_empty())
}

// ── 种子数据 ──────────────────────────────────────────────────────────────

/// 演示商品种子条目
struct SeedProduct {
    slug: &'static str,
    title: &'static str,
    description: &'static str,
    price_cents: i64,
    stock: i64,
}

/// 12 个演示商品（英文 slug，picsum.photos 占位图）
const SEED_PRODUCTS: [SeedProduct; 12] = [
    SeedProduct {
        slug: "wireless-headphones",
        title: "Wireless Headphones Pro",
        description: "## 沉浸式无线聆听\n\n- 主动降噪，隔绝喧嚣\n- 40 小时超长续航\n- 轻量耳罩，久戴不累\n\n> 通勤、办公、旅途皆宜。",
        price_cents: 5999,
        stock: 30,
    },
    SeedProduct {
        slug: "mechanical-keyboard",
        title: "Mechanical Keyboard 75%",
        description: "## 手感与颜值的平衡\n\n- 热插拔轴体，随心更换\n- PBT 键帽，不打油\n- 三模连接（有线 / 蓝牙 / 2.4G）\n\n> 程序员与文字工作者的利器。",
        price_cents: 7999,
        stock: 25,
    },
    SeedProduct {
        slug: "smart-desk-lamp",
        title: "Smart Desk Lamp",
        description: "## 护眼智能台灯\n\n- 无频闪全光谱灯珠\n- App 远程调光调色\n- 记忆常用场景\n\n> 深夜工作，眼睛也轻松。",
        price_cents: 3999,
        stock: 3,
    },
    SeedProduct {
        slug: "travel-backpack",
        title: "Travel Backpack 40L",
        description: "## 一个背包走天下\n\n- 40L 大容量，独立电脑仓\n- 防泼水面料\n- 背部透气减压设计\n\n> 短途旅行与通勤两相宜。",
        price_cents: 4999,
        stock: 0,
    },
    SeedProduct {
        slug: "smart-watch",
        title: "Smart Watch S2",
        description: "## 手腕上的健康管家\n\n- 心率 / 血氧 / 睡眠监测\n- 14 天长续航\n- 50 米防水\n\n> 运动与日常佩戴兼得。",
        price_cents: 12999,
        stock: 18,
    },
    SeedProduct {
        slug: "portable-coffee-maker",
        title: "Portable Coffee Maker",
        description: "## 随时随地来一杯\n\n- 手压萃取，无需电源\n- 一键清洗\n- 兼容胶囊与咖啡粉\n\n> 户外与办公室的咖啡自由。",
        price_cents: 6999,
        stock: 12,
    },
    SeedProduct {
        slug: "ceramic-mug-set",
        title: "Ceramic Mug Set (2 Pack)",
        description: "## 温暖手作感\n\n- 高温烧制陶瓷\n- 大容量 350ml\n- 可进微波炉与洗碗机\n\n> 一杯咖啡，一份宁静。",
        price_cents: 2499,
        stock: 50,
    },
    SeedProduct {
        slug: "eco-yoga-mat",
        title: "Eco Yoga Mat",
        description: "## 与地球一起呼吸\n\n- 天然橡胶材质\n- 双面防滑纹理\n- 附赠背带\n\n> 每一次拉伸都安稳。",
        price_cents: 2999,
        stock: 5,
    },
    SeedProduct {
        slug: "bluetooth-speaker",
        title: "Bluetooth Speaker Mini",
        description: "## 小身材大能量\n\n- 360° 环绕声场\n- IPX7 防水\n- 12 小时续航\n\n> 户外派对的好伙伴。",
        price_cents: 3499,
        stock: 40,
    },
    SeedProduct {
        slug: "slim-leather-wallet",
        title: "Slim Leather Wallet",
        description: "## 极简出行的答案\n\n- 头层牛皮\n- RFID 防盗刷\n- 仅 8mm 厚度\n\n> 少即是多。",
        price_cents: 1999,
        stock: 2,
    },
    SeedProduct {
        slug: "bamboo-desk-organizer",
        title: "Bamboo Desk Organizer",
        description: "## 桌面收纳美学\n\n- 天然竹材\n- 手机 / 笔 / 便签分区收纳\n- 免安装即用\n\n> 让桌面回归整洁。",
        price_cents: 2299,
        stock: 60,
    },
    SeedProduct {
        slug: "rgb-led-strip",
        title: "RGB LED Strip 5m",
        description: "## 氛围感拉满\n\n- 1600 万色可调\n- 手机 App 控制\n- 音乐律动模式\n\n> 一键切换空间气质。",
        price_cents: 1599,
        stock: 0,
    },
];

/// 启动种子：product 表为空时插入 12 个演示商品
pub async fn seed_products() -> Result<(), String> {
    let db = db::get_db();
    let mut res = db
        .query("SELECT count() FROM product GROUP ALL")
        .await
        .map_err(|e| e.to_string())?;
    let count: Option<u64> = res.take((0, "count")).map_err(|e| e.to_string())?;
    if count.unwrap_or(0) > 0 {
        return Ok(());
    }
    for item in SEED_PRODUCTS {
        let image = format!("https://picsum.photos/seed/{}/800/600", item.slug);
        let id = db::new_record_id("product");
        db.query(
            "CREATE product CONTENT { id: $id, slug: $slug, title: $title, description: $description, price_cents: $price, stock: $stock, image: $image, status: $status, created_at: time::now() }",
        )
        .bind(("id", db::record_id(&id).map_err(|e| e.to_string())?))
        .bind(("slug", item.slug.to_string()))
        .bind(("title", item.title.to_string()))
        .bind(("description", item.description.to_string()))
        .bind(("price", item.price_cents))
        .bind(("stock", item.stock))
        .bind(("image", image))
        .bind(("status", PRODUCT_STATUS_ACTIVE))
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
