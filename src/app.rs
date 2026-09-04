mod account;
mod admin;
mod checkout;
mod forgot_password;
mod home;
mod media;
mod orders;
mod products;
mod register;
mod sign_in;
mod users;
mod webhook;

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

    module_router!()
        .discover()
        .cookies()
        .sessions(crate::common::session::session_config())
        .assets(AssetBundle::load().unwrap())
        .app_context(mail_config)
        .layer(crate::common::locale_redirect::LocaleRedirect)
        .layer(crate::common::login_guard::LoginGuard)
        .layer(crate::common::admin_guard::AdminGuard)
        .layer(
            BodyLimit::max(crate::common::media::UPLOAD_BODY_LIMIT)
                .at("/{locale}/admin/upload"),
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
    // 根路径内容与 /{locale} 重复：声明当次检测语言的规范 URL（自动分配，不固定）
    let canonical = if topcoat::router::request::parts(cx).uri.path() == "/" {
        Some(format!("/{locale}"))
    } else {
        None
    };
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
                <title>(loader::t(&locale, "site_name"))</title>
                if let Some(ref url) = canonical {
                    <link rel="canonical" href=(url.clone())>
                }
                <meta
                    name="description"
                    content=(loader::t(&locale, "site_slogan_ext"))
                >
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
                        username: username,
                        is_admin: is_admin,
                        csrf: csrf,
                        path_and_query: path_and_query
                    )
                    <main class="flex-1">(slot)</main>
                    components::footer::Footer(locale: locale.to_string())
                </div>
            </body>
        </html>
    })
}
