#![allow(non_snake_case)]

use crate::common::auth;
use crate::common::constant::{USER_STATUS_ACTIVE, USER_STATUS_BANNED};
use crate::common::markdown;
use crate::components::badge::{BadgeVariant, badge};
use crate::components::card::card;
use crate::components::status_badge::warning_badge;
use crate::db::users;
use crate::i18n::loader;
use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param_segment},
    view::{attributes, view},
};

/// 用户资料页（公开只读）：用户名、状态徽章、简介（Markdown）；邮箱仅本人可见
#[page("/{locale}/users/{username}")]
pub async fn user_profile(cx: &Cx) -> Result {
    let locale = path_param_segment(cx, "locale");
    let username = path_param_segment(cx, "username");
    let user = users::get_user_profile(username).await.ok().flatten();
    // 邮箱属于隐私：仅当访问者就是本人时展示
    let is_self = auth::current_user(cx).await.as_deref() == Some(username);
    view! {
        <main class="max-w-2xl mx-auto px-4 py-8">
            if let Some(ref u) = user {
                let status_key = match u.status {
                    USER_STATUS_BANNED => "user_status_banned",
                    USER_STATUS_ACTIVE => "user_status_active",
                    _ => "user_status_pending",
                };
                card(
                    attrs: attributes! { class="p-6" },
                    <div class="flex items-center gap-3 mb-4">
                        <h1 class="text-2xl font-bold text-foreground">
                            (u.username.clone())
                        </h1>
                        if u.status == USER_STATUS_BANNED {
                            badge(
                                variant: BadgeVariant::Destructive,
                                attrs: attributes! {},
                                (loader::t(&locale, status_key))
                            )
                        } else if u.status == USER_STATUS_ACTIVE {
                            badge(
                                variant: BadgeVariant::Secondary,
                                attrs: attributes! {},
                                (loader::t(&locale, status_key))
                            )
                        } else {
                            warning_badge(
                                attrs: attributes! {},
                                (loader::t(&locale, status_key))
                            )
                        }
                    </div>
                    if is_self {
                        <p class="text-sm text-muted-foreground mb-4">
                            (u.email.clone())
                        </p>
                    }
                    if !u.introduction.is_empty() {
                        <div class="prose prose-sm max-w-none text-foreground">
                            (topcoat::view::Unescaped::new_unchecked(
                                markdown::render_md(&u.introduction),
                            ))
                        </div>
                    } else {
                        <p class="text-muted-foreground">
                            (loader::t(&locale, "no_data"))
                        </p>
                    }
                )
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
        </main>
    }
}
