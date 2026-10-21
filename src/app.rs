mod account;
mod admin;
mod arcs;
mod forgot_password;
mod home;
mod media;
mod notify;
mod register;
mod sign_in;
mod upload;
mod users;

use tokio::sync::broadcast;

use crate::common::{auth, config};
use crate::components;
use crate::i18n::loader;
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    cookie::RouterBuilderCookieExt,
    font::{Font, fontsource::fontsource_font},
    router::{Router, RouterBuilderDiscoverExt, Slot, layout, module_router, tower::TowerLayer, BodyLimit},
    runtime::RouterBuilderRuntimeExt,
    session::RouterBuilderSessionExt,
    tailwind,
    view::{View, view},
};

/// 主题字体
const SANS_FONT: Font = fontsource_font!(GEIST, host: Asset);

/// favicon（构建期打包进 bundle，不再依赖无路由的 /favicon.svg 直链）
const FAVICON: topcoat::asset::Asset = topcoat::asset::asset!("public/favicon.svg");

pub fn router() -> Router {
    let cfg = config::config();
    let smtp = topcoat::mail::SmtpTransport::relay(&cfg.email_smtp)
        .expect("SMTP 连接失败")
        .credentials(&cfg.email_username, &cfg.email_password)
        .build();
    let mail_config = topcoat::mail::MailConfig::builder().transport(smtp).build();
    let (notify_tx, _) = broadcast::channel::<String>(32);

    module_router!()
        .discover()
        .runtime()
        .cookies()
        .sessions(crate::common::session::session_config())
        .assets(AssetBundle::load().unwrap())
        .app_context(mail_config)
        .app_context(notify_tx)
        .layer(crate::common::locale_redirect::LocaleRedirect)
        .layer(crate::common::admin_guard::AdminGuard)
        .layer(
            BodyLimit::max(crate::common::media::UPLOAD_BODY_LIMIT)
                .at("/{locale}/upload"),
        )
        .layer(TowerLayer::new(
            tower_http::trace::TraceLayer::new_for_http(),
        ))
        .build()
}

#[layout("/")]
async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let locale = loader::locale_from_path(cx);
    let (signed_in, username) = match auth::current_user(cx).await {
        Some(u) => (true, Some(u)),
        None => (false, None),
    };
    let is_admin = if signed_in { auth::is_admin(cx).await } else { false };
    // 签出表单的 CSRF token（未登录不渲染表单，无会话则为空串）
    let csrf = if signed_in {
        crate::common::session::ensure_csrf_token(cx).await.unwrap_or_default()
    } else {
        String::new()
    };
    let path_and_query = topcoat::router::request::parts(cx)
        .uri
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_default();
    // 页面元数据：title/description/og 图/og 类型/是否 noindex（与详情页共享 memoize 查询）
    let (title, description, og_image, og_type, noindex) = page_meta(cx, &locale).await;
    let title_og = title.clone();
    let desc_meta = description.clone();
    // 规范 URL 与语言备用：全页自引用（根路径解析到当次检测语言），绝对地址、不含查询串
    let path_only = if topcoat::router::request::parts(cx).uri.path() == "/" {
        format!("/{locale}")
    } else {
        topcoat::router::request::parts(cx).uri.path().to_string()
    };
    let domain = &config::config().domain;
    let canonical_url = format!("https://{domain}{path_only}");
    let canonical_og = canonical_url.clone();
    let alt_en = format!("https://{domain}{}", loader::swap_locale(&path_only, "en"));
    let alt_en_x = alt_en.clone();
    let alt_zh = format!("https://{domain}{}", loader::swap_locale(&path_only, "zh"));
    // og:image 相对路径补全为绝对地址
    let og_image = og_image.map(|img| {
        if img.starts_with("http") {
            img
        } else {
            format!("https://{domain}{img}")
        }
    });
    let og_locale = if locale == "zh" { "zh_CN" } else { "en_US" };
    let og_locale_alt = if locale == "zh" { "en_US" } else { "zh_CN" };
    // 语言记忆：访问任何语言路径页即刷新 lang cookie（切换语言即刻持久化）
    // 必须在布局内做——中间件层运行于 cookie jar 注册之前，那里调 cookies 会 panic
    if let Some(first) = topcoat::router::request::parts(cx)
        .uri
        .path()
        .split('/')
        .nth(1)
        .filter(|f| loader::is_supported(f))
    {
        loader::remember(cx, first);
    }
    Ok(view! {
        <!DOCTYPE html>
        <html lang=(locale.as_str())>
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>(title)</title>
                <link rel="canonical" href=(canonical_url)>
                <meta name="description" content=(desc_meta)>
                if noindex {
                    <meta name="robots" content="noindex,follow">
                }
                <link rel="alternate" hreflang="en" href=(alt_en)>
                <link rel="alternate" hreflang="zh" href=(alt_zh)>
                <link rel="alternate" hreflang="x-default" href=(alt_en_x)>
                <meta property="og:title" content=(title_og)>
                <meta property="og:description" content=(description)>
                <meta property="og:url" content=(canonical_og)>
                <meta property="og:type" content=(og_type)>
                if let Some(img) = og_image {
                    <meta property="og:image" content=(img)>
                }
                <meta property="og:locale" content=(og_locale)>
                <meta property="og:locale:alternate" content=(og_locale_alt)>
                <meta name="twitter:card" content="summary_large_image">
                <link rel="icon" type="image/svg+xml" href=(FAVICON)>
                topcoat::dev::script()
                topcoat::runtime::script()
                topcoat::font::link(font: SANS_FONT)
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
                <script>
                    "(function(){var d=localStorage.getItem('theme');var e=document.documentElement;var v=d||(matchMedia('(prefers-color-scheme:dark)').matches?'dark':'light');e.classList.add(v)})();function setTheme(b){var e=document.documentElement;if(b){e.classList.add('dark');e.classList.remove('light');localStorage.setItem('theme','dark')}else{e.classList.remove('dark');e.classList.add('light');localStorage.setItem('theme','light')}}function toggleTheme(){setTheme(!document.documentElement.classList.contains('dark'))}"
                </script>
            </head>
            <body class="min-h-screen bg-background text-foreground">
                <div class="flex flex-col min-h-screen">
                    components::nav::Nav(
                        locale: locale.to_string(),
                        signed_in: signed_in,
                        username: username.clone(),
                        is_admin: is_admin,
                        csrf: csrf,
                        path_and_query: path_and_query
                    )
                    <div class="flex flex-1">
                        components::sidebar::Sidebar(
                            locale: locale.to_string(),
                            signed_in: signed_in,
                            username: username
                        )
                        <main class="flex-1">(slot)</main>
                    </div>
                    components::footer::Footer(locale: locale.to_string())
                </div>
                if signed_in {
                    <script>
                        "(function(){var es=new EventSource('/{locale}/notify');es.onmessage=function(e){var el=document.createElement('div');el.className='fixed top-4 left-1/2 -translate-x-1/2 z-50 bg-surface border border-border rounded-lg px-4 py-2 text-sm text-foreground shadow-lg';el.textContent=e.data;document.body.appendChild(el);setTimeout(function(){el.style.opacity='0';el.style.transition='opacity .3s';setTimeout(function(){el.remove()},300)},3000)}})();"
                    </script>
                }
            </body>
        </html>
    })
}

/// 页面元数据：title / description / og 图 / og 类型 / 是否 noindex
async fn page_meta(cx: &Cx, locale: &str) -> (String, String, Option<String>, &'static str, bool) {
    let brand = loader::t(locale, "site_name");
    let slogan = loader::t(locale, "site_slogan_ext").to_string();
    let path = topcoat::router::request::parts(cx).uri.path();
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let key = segs.get(1).copied().unwrap_or("");
    let sub = segs.get(2).copied();
    if key.is_empty() {
        return (
            format!("{brand} - {}", loader::t(locale, "site_slogan")),
            slogan, None, "website", false,
        );
    }
    // 内容详情：标题/摘要/缩略图（与详情页自身查询共享 memoize 缓存）
    if key == "arc" && let Some(slug) = sub {
        return match crate::db::arcs::get_arc_by_slug(cx, slug).await {
            Ok(Some(a)) => {
                let text = a
                    .summary
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .or(a.body.as_deref())
                    .unwrap_or("");
                (
                    format!("{} - {brand}", a.title),
                    crate::common::markdown::meta_description(text),
                    a.thumbnail.clone(),
                    "article",
                    false,
                )
            }
            _ => (brand.to_string(), slogan, None, "website", false),
        };
    }
    // 列表页：全量或按类型
    if key == "arcs" {
        let name = match sub {
            Some("video") => loader::t(locale, "post_type_video"),
            Some("article") => loader::t(locale, "post_type_article"),
            Some("photo") => loader::t(locale, "post_type_photo"),
            _ => loader::t(locale, "all"),
        };
        return (format!("{name} - {brand}"), slogan, None, "website", false);
    }
    // 用户页：个人主页可收录，工具页 noindex
    if key == "users" {
        return match sub {
            Some(username)
                if !matches!(username, "forgot-password" | "reset-password" | "resend" | "activate") =>
            {
                (format!("{username} - {brand}"), slogan, None, "website", false)
            }
            Some("forgot-password") => (
                format!("{} - {brand}", loader::t(locale, "forgot_password_title")),
                slogan, None, "website", true,
            ),
            Some("reset-password") => (
                format!("{} - {brand}", loader::t(locale, "reset_password_title")),
                slogan, None, "website", true,
            ),
            Some("resend") => (
                format!("{} - {brand}", loader::t(locale, "resend_title")),
                slogan, None, "website", true,
            ),
            _ => (brand.to_string(), slogan, None, "website", true),
        };
    }
    let (name, noindex) = match key {
        "upload" => (loader::t(locale, "upload_title").to_string(), true),
        "sign-in" => (loader::t(locale, "sign_in").to_string(), true),
        "register" => (loader::t(locale, "register").to_string(), true),
        "account" => (loader::t(locale, "account_change_password").to_string(), true),
        "admin" => (loader::t(locale, "admin_users").to_string(), true),
        _ => return (brand.to_string(), slogan, None, "website", false),
    };
    (format!("{name} - {brand}"), slogan, None, "website", noindex)
}
