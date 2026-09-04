#![allow(non_snake_case)]

use crate::common::auth;
use crate::common::config;
use crate::common::format::format_cents;
use crate::common::form;
use crate::common::markdown;
use crate::common::session;
use crate::components;
use crate::components::button::{ButtonVariant, button, button_variants};
use crate::components::card::card;
use crate::components::input::input;
use crate::db::products;
use crate::i18n::loader;
use crate::models::page::PageInfo;
use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param_segment, query_params},
    runtime::{Event, shard},
    view::{View, attributes, view},
};

#[query_params]
pub struct ProductQuery {
    pub page: Option<u64>,
    pub q: Option<String>,
}

/// 商品列表页：signal 搜索 + shard 网格 + 分页
#[page("/{locale}/products")]
pub async fn products_list(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let params = query_params::<ProductQuery>(cx).ok();
    let initial_q = params
        .as_ref()
        .and_then(|p| p.q.as_deref())
        .unwrap_or("")
        .to_string();
    let list_error = form::error_message(cx, &locale, &["invalid"]);
    Ok(view! {
        signal query = initial_q;
        signal locale_sig = locale.to_string();

        <div class="max-w-6xl mx-auto px-4 py-8">
            <h1 class="text-xl font-bold mb-6 text-foreground">
                (loader::t(&locale, "nav_products"))
            </h1>
            if let Some(ref msg) = list_error {
                <p class="text-red-500 text-sm mb-4">(msg.clone())</p>
            }
            <div class="flex flex-wrap items-center gap-3 mb-6">
                <div class="flex-1 flex items-center gap-2 max-w-md">
                    input(
                        attrs: attributes! {
                            type="text"
                            placeholder=(loader::t(&locale, "search_placeholder"))
                            class="flex-1"
                            :value=$(query.get())
                            @input=$(|e: Event| query.set(e.target.value))
                        }
                    )
                    button(
                        variant: ButtonVariant::Secondary,
                        attrs: attributes! {
                            type="button"
                            @click=$(|_e: Event| query.set("".to_owned()))
                        },
                        (loader::t(&locale, "all"))
                    )
                </div>
            </div>

            product_grid(locale: $(locale_sig.get()), query: $(query.get()))
        </div>
    })
}

/// 商品网格：shard 在客户端按信号重渲染时运行于自身请求上下文，
/// 页面路径参数不可用，故 locale 以参数显式传入
#[shard]
async fn product_grid(cx: &Cx, locale: String, query: String) -> Result<impl View> {
    let loc = locale.clone();
    let params = query_params::<ProductQuery>(cx).ok();
    let page = params.as_ref().and_then(|p| p.page).unwrap_or(1).max(1);
    let search_str = if query.trim().is_empty() {
        None
    } else {
        Some(query.trim())
    };
    let page_size = config::config().page_size as u64;
    let total = products::count_products(search_str).await.unwrap_or(0);
    let page_info = PageInfo::new(total, page, page_size);
    let product_list = products::list_products(search_str, page_info.current_page, page_size)
        .await
        .unwrap_or_default();
    let n = product_list.len();
    let locales: Vec<String> = std::iter::repeat(loc.clone()).take(n).collect();
    let base_url = match search_str {
        Some(q) => format!("/{locale}/products?q={q}"),
        None => format!("/{locale}/products"),
    };
    Ok(view! {
        if product_list.is_empty() {
            <div class="text-center py-16">
                <p class="text-base text-muted-foreground">
                    (loader::t(&locale, "no_data"))
                </p>
            </div>
        } else {
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                for (item, lc) in product_list.into_iter().zip(locales) {
                    components::product_card::ProductCard(locale: lc, product: item)
                }
            </div>
            components::pagination::Pagination(
                locale: loc,
                page_info: page_info,
                base_url: base_url
            )
        }
    })
}

/// 商品详情页：左图右购买面板（桌面）/ 单列（移动）
#[page("/{locale}/products/{slug}")]
pub async fn product_detail(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let slug = path_param_segment(cx, "slug");
    let product = products::get_product_by_slug(&slug).await.ok().flatten();
    let body_html = product
        .as_ref()
        .map(|p| markdown::render_md(&p.description));
    let signed_in = auth::current_user(cx).await.is_some();
    // Buy Now 表单的 CSRF token（仅登录态渲染表单）
    let csrf = if signed_in {
        session::ensure_csrf_token(cx).await.unwrap_or_default()
    } else {
        String::new()
    };
    let checkout_error = form::error_message(cx, &locale, &["payment", "invalid", "stock"]);
    let current_path = topcoat::router::request::parts(cx)
        .uri
        .path()
        .to_string();
    Ok(view! {
        <div class="max-w-6xl mx-auto px-4 py-8">
            if let Some(ref p) = product {
                <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
                    // 商品图：浅底铁律，bg-white 包裹
                    <div
                        class="aspect-4/3 bg-white rounded-lg border border-border overflow-hidden"
                    >
                        if let Some(ref img) = p.image {
                            <img
                                src=(img.clone())
                                alt=(p.title.clone())
                                class="w-full h-full object-cover"
                            >
                        }
                    </div>
                    // 购买面板
                    <div class="flex flex-col gap-4">
                        <h1 class="text-2xl font-bold text-foreground">
                            (p.title.clone())
                        </h1>
                        <div
                            class="text-3xl font-bold text-blue-600 dark:text-blue-400"
                        >
                            (format_cents(p.price_cents))
                        </div>
                        <div>
                            components::product_card::StockBadge(
                                locale: locale.to_string(),
                                stock: p.stock
                            )
                        </div>
                        if let Some(ref msg) = checkout_error {
                            <p class="text-red-500 text-sm">(msg.clone())</p>
                        }
                        if signed_in {
                            <form method="POST" action=(format!("/{locale}/checkout"))>
                                <input
                                    type="hidden"
                                    name="product_id"
                                    value=(p.id.clone())
                                >
                                <input
                                    type="hidden"
                                    name="next"
                                    value=(current_path.clone())
                                >
                                crate::components::csrf::CsrfField(token: csrf)
                                button(
                                    variant: ButtonVariant::Primary,
                                    attrs: attributes! { type="submit" class="w-full justify-center" },
                                    (loader::t(&locale, "product_buy_now"))
                                )
                            </form>
                        } else {
                            <a
                                href=(format!("/{locale}/sign-in?next={}", current_path))
                                class=(button_variants(
                                    ButtonVariant::Primary,
                                    crate::components::button::ButtonSize::Md,
                                ))
                            >
                                (loader::t(&locale, "product_buy_now"))
                            </a>
                        }
                    </div>
                </div>
                // Markdown 描述：render_md 渲染 + 编辑器同款展示容器
                <div class="mt-8">
                    <h2 class="text-lg font-bold text-foreground mb-4">
                        (loader::t(&locale, "product_description"))
                    </h2>
                    card(
                        attrs: attributes! { class="p-6 prose-sm" },
                        (topcoat::view::Unescaped::new_unchecked(
                            body_html.unwrap_or_default(),
                        ))
                    )
                </div>
                // 描述内嵌 <video src="…m3u8"> 经 hls.js 接管播放
                crate::components::hls_player::HlsScan()
            } else {
                (topcoat::router::StatusCode::NOT_FOUND)
                <div class="text-center py-16">
                    <h1
                        class="text-7xl font-bold text-blue-600 dark:text-blue-400 mb-4"
                    >
                        "404"
                    </h1>
                    <p class="text-muted-foreground mb-4">
                        (loader::t(&locale, "page_error_404"))
                    </p>
                    <a
                        href=(format!("/{locale}/products"))
                        class="text-blue-600 dark:text-blue-400 hover:underline"
                    >
                        (loader::t(&locale, "nav_products"))
                    </a>
                </div>
            }
        </div>
    })
}
