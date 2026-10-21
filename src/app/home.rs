#![allow(non_snake_case)]

use crate::common::config;
use crate::common::icons;
use crate::components;
use crate::db::arcs;
use crate::i18n::loader;
use crate::models::arc::Arc;
use topcoat::{
    Result,
    context::Cx,
    icon::icon,
    router::path_param_segment,
    view::{View, component, view},
};

/// 根路径：不跳转，以检测语言直接渲染首页；后续操作均在语言路径下
#[topcoat::router::page("/")]
pub async fn home_root(cx: &Cx) -> Result<impl View> {
    let locale = loader::detect(cx);
    Ok(view! { HomeContent(locale: locale) })
}

/// 语言路径首页（路径段是唯一权威语言入口）
#[topcoat::router::page("/{locale}")]
pub async fn locale_home(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale").to_string();
    Ok(view! { HomeContent(locale: locale) })
}

#[component]
async fn HomeContent(cx: &Cx, locale: String) -> Result<impl View> {
    let (videos, articles, photos) = arcs::get_home_arcs(cx, 12).await.unwrap_or_default();
    // JSON-LD 结构化数据：站点信息
    let site_ld = crate::common::markdown::json_ld(&serde_json::json!({
        "@context": "https://schema.org",
        "@type": "WebSite",
        "name": loader::t(&locale, "site_name"),
        "url": format!("https://{}", config::config().domain),
    }));
    Ok(view! {
        <main class="max-w-6xl mx-auto px-4 py-8">
            <div class="mb-10 text-center">
                <h1 class="text-3xl font-bold text-foreground mb-3">
                    (loader::t(&locale, "site_slogan"))
                </h1>
                <p class="text-sm text-muted-foreground max-w-2xl mx-auto">
                    (loader::t(&locale, "site_slogan_ext"))
                </p>
            </div>
            <script type="application/ld+json">
                (topcoat::view::Unescaped::new_unchecked(site_ld))
            </script>
            if videos.is_empty() && articles.is_empty() && photos.is_empty() {
                <div class="text-center py-16">
                    <p class="text-base text-muted-foreground">
                        (loader::t(&locale, "no_data"))
                    </p>
                </div>
            } else {
                let loc = locale.clone();
                HomeSection(
                    locale: loc.clone(),
                    section_icon: icons::VIDEO,
                    title: loader::t(&locale, "post_type_video").to_string(),
                    arcs: videos
                )
                HomeSection(
                    locale: loc.clone(),
                    section_icon: icons::PENCIL,
                    title: loader::t(&locale, "post_type_article").to_string(),
                    arcs: articles
                )
                HomeSection(
                    locale: loc,
                    section_icon: icons::CAMERA,
                    title: loader::t(&locale, "post_type_photo").to_string(),
                    arcs: photos
                )
            }
        </main>
    })
}

#[component]
async fn HomeSection(
    locale: String,
    section_icon: topcoat::icon::IconData,
    title: String,
    arcs: Vec<Arc>,
) -> Result<impl View> {
    let n = arcs.len();
    let locales: Vec<String> = std::iter::repeat(locale).take(n).collect();
    Ok(view! {
        if !arcs.is_empty() {
            <section class="mb-12">
                <h2
                    class="text-lg font-semibold text-foreground border-b border-blue-200 dark:border-blue-800 pb-2 mb-4 flex items-center gap-2"
                >
                    <span class="text-blue-500">icon(data: section_icon)</span>
                    (title)
                </h2>
                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                    for (arc, loc) in arcs.iter().zip(locales) {
                        components::arc_preview::ArcPreview(
                            locale: loc,
                            arc: arc.clone()
                        )
                    }
                </div>
            </section>
        }
    })
}
