#![allow(non_snake_case)]

use crate::components;
use crate::db::products;
use crate::i18n::loader;
use topcoat::{
    Result,
    context::Cx,
    router::path_param_segment,
    view::{component, view},
};

/// 首页最新商品数量（试水：一屏 8 张卡片，不重复列表页分页能力）
const HOME_LATEST_COUNT: u64 = 8;

/// 根路径：不跳转，以检测语言直接渲染首页；后续操作均在语言路径下
#[topcoat::router::page("/")]
pub async fn home_root(cx: &Cx) -> Result {
    let locale = loader::detect(cx);
    view! { HomeContent(locale: locale) }
}

/// 语言路径首页（路径段是唯一权威语言入口）
#[topcoat::router::page("/{locale}")]
pub async fn locale_home(cx: &Cx) -> Result {
    let locale = path_param_segment(cx, "locale").to_string();
    view! { HomeContent(locale: locale) }
}

/// 首页内容：价值主张 + 最新上架商品（复用 ProductCard，无信号不引 shard）
#[component]
async fn HomeContent(locale: String) -> Result {
    let latest = products::list_products(None, 1, HOME_LATEST_COUNT)
        .await
        .unwrap_or_default();
    let locales: Vec<String> = std::iter::repeat_n(locale.clone(), latest.len()).collect();
    view! {
        <section class="max-w-6xl mx-auto px-4 pt-12 pb-2 text-center">
            <h1 class="text-3xl font-bold text-foreground">
                (loader::t(&locale, "site_slogan"))
            </h1>
            <p class="mt-3 text-muted-foreground">
                (loader::t(&locale, "site_slogan_ext"))
            </p>
        </section>
        if latest.is_empty() {
            ""
        } else {
            <section class="max-w-6xl mx-auto px-4 py-8">
                <div class="flex items-center justify-between mb-4">
                    <h2 class="text-lg font-bold text-foreground">
                        (loader::t(&locale, "home_latest"))
                    </h2>
                    <a
                        href=(format!("/{locale}/products"))
                        class="text-sm text-blue-600 dark:text-blue-400 hover:underline no-underline"
                    >
                        (loader::t(&locale, "home_view_all"))
                    </a>
                </div>
                <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-4">
                    for (product, lc) in latest.into_iter().zip(locales) {
                        components::product_card::ProductCard(
                            locale: lc,
                            product: product
                        )
                    }
                </div>
            </section>
        }
    }
}
