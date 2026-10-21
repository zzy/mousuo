#![allow(non_snake_case)]

use serde::Deserialize;

use crate::common::{auth, config, form, session};
use crate::components;
use crate::components::badge::{BadgeVariant, badge, badge_variants};
use crate::components::button::{ButtonVariant, button};
use crate::components::card::card;
use crate::components::csrf::CsrfField;
use crate::components::input::input;
use crate::db::arcs;
use crate::i18n::loader;
use crate::models::page::PageInfo;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form, error::{bad_request, forbidden}, page, path_param_segment, query_params,
        response::Response,
    },
    runtime::{Event, shard, signal},
    view::{View, attributes, class, component, view},
};

#[query_params]
pub struct ArcQuery {
    pub page: Option<u64>,
    pub q: Option<String>,
}

/// 列表页：全类型
#[page("/{locale}/arcs")]
pub async fn arcs_list() -> Result<impl View> {
    Ok(view! { ArcsContent(type_key: String::new()) })
}

/// 列表页：按类型（路径分段；未知类型视同未指定，显示全部）
#[page("/{locale}/arcs/{type}")]
pub async fn arcs_list_typed(cx: &Cx) -> Result<impl View> {
    let raw = path_param_segment(cx, "type");
    let type_key = match raw {
        "video" | "article" | "photo" => raw.to_string(),
        _ => String::new(),
    };
    Ok(view! { ArcsContent(type_key: type_key) })
}

/// 列表页共用渲染：类型过滤 + 前端即时搜索（type_key 空串 = 全类型）
#[component]
async fn ArcsContent(cx: &Cx, type_key: String) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let params = query_params::<ArcQuery>(cx).ok();
    let initial_q = params
        .as_ref()
        .and_then(|p| p.q.as_deref())
        .unwrap_or("")
        .to_string();
    let query = signal(cx, move || initial_q);
    Ok(view! {
        <div class="max-w-6xl mx-auto px-4 py-8">
            <h1 class="text-xl font-bold mb-6 text-foreground">
                (loader::t(
                    &locale,
                    match type_key.as_str() {
                        "video" => "post_type_video",
                        "article" => "post_type_article",
                        "photo" => "post_type_photo",
                        _ => "nav_article",
                    },
                ))
            </h1>
            <div class="flex flex-wrap items-center gap-3 mb-6">
                <a
                    href=(format!("/{locale}/arcs"))
                    class=(class!(
                        "no-underline",
                        badge_variants(
                            if type_key.is_empty() {
                                BadgeVariant::Primary
                            } else {
                                BadgeVariant::Secondary
                            },
                        ),
                    ))
                >
                    (loader::t(&locale, "all"))
                </a>
                <a
                    href=(format!("/{locale}/arcs/video"))
                    class=(class!(
                        "no-underline",
                        badge_variants(
                            if type_key == "video" {
                                BadgeVariant::Primary
                            } else {
                                BadgeVariant::Secondary
                            },
                        ),
                    ))
                >
                    (loader::t(&locale, "post_type_video"))
                </a>
                <a
                    href=(format!("/{locale}/arcs/article"))
                    class=(class!(
                        "no-underline",
                        badge_variants(
                            if type_key == "article" {
                                BadgeVariant::Primary
                            } else {
                                BadgeVariant::Secondary
                            },
                        ),
                    ))
                >
                    (loader::t(&locale, "post_type_article"))
                </a>
                <a
                    href=(format!("/{locale}/arcs/photo"))
                    class=(class!(
                        "no-underline",
                        badge_variants(
                            if type_key == "photo" {
                                BadgeVariant::Primary
                            } else {
                                BadgeVariant::Secondary
                            },
                        ),
                    ))
                >
                    (loader::t(&locale, "post_type_photo"))
                </a>
                <div class="flex-1 flex items-center gap-2 max-w-md ml-auto">
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
                        variant: ButtonVariant::Primary,
                        attrs: attributes! {
                            type="button"
                            @click=$(|_e: Event| query.set("".to_owned()))
                        },
                        (loader::t(&locale, "all"))
                    )
                </div>
            </div>

            arc_grid(query: $(query.get()), type_key: $(type_key))
        </div>
    })
}

#[shard]
async fn arc_grid(cx: &Cx, query: String, type_key: String) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let loc = locale.to_string();
    let params = query_params::<ArcQuery>(cx).ok();
    let type_filter = if type_key.is_empty() {
        None
    } else {
        Some(type_key)
    };
    let page = params.as_ref().and_then(|p| p.page).unwrap_or(1).max(1);
    let search_str = if query.trim().is_empty() {
        None
    } else {
        Some(query.trim())
    };
    let page_size = config::config().page_size as u64;
    let total = arcs::count_arcs(type_filter.as_deref(), search_str)
        .await
        .unwrap_or(0);
    let page_info = PageInfo::new(total, page, page_size);
    let arc_list = arcs::get_arcs(
        type_filter.as_deref(),
        search_str,
        page_info.current_page,
        page_size,
    )
    .await
    .unwrap_or_default();
    let n = arc_list.len();
    let locales: Vec<String> = std::iter::repeat(loc.clone()).take(n).collect();
    let base_url = match type_filter.as_deref() {
        Some(t) => format!("/{locale}/arcs/{t}"),
        None => format!("/{locale}/arcs"),
    };
    let base_url = match search_str {
        Some(q) => format!("{base_url}?q={q}"),
        None => base_url,
    };
    Ok(view! {
        if arc_list.is_empty() {
            <div class="text-center py-16">
                <p class="text-base text-muted-foreground">
                    (loader::t(&locale, "no_data"))
                </p>
            </div>
        } else {
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                for (item, lc) in arc_list.into_iter().zip(locales) {
                    components::arc_preview::ArcPreview(locale: lc, arc: item)
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

#[page("/{locale}/arc/{slug}")]
pub async fn arc_detail(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let slug = path_param_segment(cx, "slug");
    let found = arcs::get_arc_by_slug(cx, &slug).await.ok().flatten();
    let body_html = found
        .as_ref()
        .and_then(|e| e.body.as_deref())
        .map(|b| crate::common::markdown::render_md(b));
    let topic_list: Vec<String> = found
        .as_ref()
        .and_then(|e| e.topics.as_deref())
        .map(|t| {
            t.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    // 删除入口：仅作者本人或管理员可见（管理员可代删）
    let current = auth::current_user(cx).await;
    let is_admin = if current.is_some() { auth::is_admin(cx).await } else { false };
    let can_delete = found
        .as_ref()
        .and_then(|a| a.author_name.as_deref())
        .zip(current.as_deref())
        .map_or(false, |(author, user)| author == user)
        || is_admin;
    let csrf = if can_delete {
        session::ensure_csrf_token(cx).await.unwrap_or_default()
    } else {
        String::new()
    };
    // JSON-LD 结构化数据：图文富媒体结果
    let article_ld = found.as_ref().map(|a| {
        crate::common::markdown::json_ld(&serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Article",
            "headline": a.title,
            "image": a.thumbnail,
            "datePublished": a.created_at,
            "author": a.author_name.as_deref().map(|n| serde_json::json!({
                "@type": "Person",
                "name": n,
            })),
        }))
    });
    Ok(view! {
        <div class="max-w-4xl mx-auto px-4 py-8">
            if let Some(ref arc) = found {
                <div class="mb-4">
                    badge(
                        variant: BadgeVariant::Primary,
                        attrs: attributes! {},
                        (loader::t(
                            &locale,
                            match arc.arc_type.as_str() {
                                "video" => "post_type_video",
                                "photo" => "post_type_photo",
                                _ => "post_type_article",
                            },
                        ))
                    )
                </div>
                <h1 class="text-2xl font-bold text-foreground mb-4">
                    (arc.title.clone())
                </h1>
                <div class="flex items-center gap-3 text-sm text-muted-foreground mb-6">
                    <span>(arc.author_name.as_deref().unwrap_or(""))</span>
                    <span class="text-muted-foreground">"|"</span>
                    <span>(&arc.created_at[..10.min(arc.created_at.len())])</span>
                    <span class="text-muted-foreground">"|"</span>
                    <span>
                        (arc.view_count)
                        " "
                        (loader::t(&locale, "views"))
                    </span>
                </div>
                if can_delete {
                    <form
                        method="POST"
                        action=(format!("/{locale}/arc/delete"))
                        class="mb-6"
                    >
                        <input type="hidden" name="id" value=(arc.id.clone())>
                        CsrfField(token: csrf.clone())
                        button(
                            variant: ButtonVariant::Destructive,
                            attrs: attributes! {
                                type="submit"
                                onclick=(format!(
                                    "return confirm('{}')",
                                    loader::t(&locale, "arc_delete_confirm"),
                                ))
                            },
                            (loader::t(&locale, "arc_delete"))
                        )
                    </form>
                }
                if let Some(ref media_url) = arc.media_url {
                    if arc.arc_type.as_str() == "video" {
                        <div
                            class="aspect-video bg-black rounded-lg overflow-hidden mb-6"
                        >
                            crate::components::hls_player::HlsPlayer(
                                src: media_url.clone()
                            )
                        </div>
                    } else if arc.arc_type.as_str() == "photo" {
                        <div class="bg-white rounded-lg overflow-hidden mb-6">
                            <img
                                src=(media_url.clone())
                                alt=""
                                class="max-w-full h-auto"
                            >
                        </div>
                    }
                }
                if !topic_list.is_empty() {
                    <div class="flex flex-wrap gap-2 mb-6">
                        for topic in topic_list {
                            badge(
                                variant: BadgeVariant::Primary,
                                attrs: attributes! {},
                                (topic)
                            )
                        }
                    </div>
                }
                if let Some(ref html) = body_html {
                    card(
                        attrs: attributes! { class="p-6 prose-sm" },
                        (topcoat::view::Unescaped::new_unchecked(html.clone()))
                    )
                }
                crate::components::hls_player::HlsScan()
                if let Some(ld) = article_ld {
                    <script type="application/ld+json">
                        (topcoat::view::Unescaped::new_unchecked(ld))
                    </script>
                }
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
                        href=(format!("/{locale}"))
                        class="text-blue-600 dark:text-blue-400 hover:underline"
                    >
                        (loader::t(&locale, "go_home"))
                    </a>
                </div>
            }
        </div>
    })
}

#[derive(Deserialize)]
struct ArcDeleteForm {
    id: String,
    #[serde(default)]
    csrf_token: String,
}

/// 删除内容：CSRF + 登录 + 作者本人或管理员（管理员可代删）
#[topcoat::router::route(POST "/{locale}/arc/delete")]
pub async fn arc_delete(cx: &Cx, Form(form): Form<ArcDeleteForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let Some(user) = auth::current_user(cx).await else {
        return Err(forbidden().into());
    };
    let Some(arc) = arcs::get_arc_by_id(&form.id).await.ok().flatten() else {
        return Ok(form::redirect(&format!("/{locale}")));
    };
    let is_admin = auth::is_admin(cx).await;
    if arc.author_name.as_deref() != Some(user.as_str()) && !is_admin {
        return Err(forbidden().into());
    }
    arcs::delete_arc(&form.id).await.map_err(|e| bad_request(e))?;
    Ok(form::redirect(&format!("/{locale}")))
}
