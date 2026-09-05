#![allow(non_snake_case)]

use crate::common::auth;
use crate::common::config;
use crate::common::constant::{USER_IS_ADMIN, USER_STATUS_ACTIVE, USER_STATUS_BANNED};
use crate::common::{form, session};
use crate::components;
use crate::components::badge::{BadgeVariant, badge};
use crate::components::button::{ButtonSize, ButtonVariant, button_variants};
use crate::components::csrf::CsrfField;
use crate::components::status_badge::warning_badge;
use crate::db::users;
use crate::i18n::loader;
use crate::models::page::PageInfo;
use crate::models::user::User;
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{content::Form, error::forbidden, page, path_param_segment, query_params, response::Response},
    view::{View, attributes, component, view},
};

#[query_params]
pub struct AdminUsersQuery {
    pub page: Option<u64>,
}

/// 用户列表（AdminGuard 层保证管理员；不展示凭据）
#[page("/{locale}/admin/users")]
pub async fn admin_users_list(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let loc = locale.to_string();
    let params = query_params::<AdminUsersQuery>(cx).ok();
    let page = params.as_ref().and_then(|p| p.page).unwrap_or(1).max(1);
    let page_size = config::config().page_size as u64;
    let total = users::count_users().await.unwrap_or(0);
    let page_info = PageInfo::new(total, page, page_size);
    let user_list = users::list_users(page_info.current_page, page_size)
        .await
        .unwrap_or_default();
    let n = user_list.len();
    let locales: Vec<String> = std::iter::repeat(loc.clone()).take(n).collect();
    // 同一会话内 CSRF token 相同：生成一次，逐行传递
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    let csrfs: Vec<String> = std::iter::repeat(csrf).take(n).collect();
    // 当前管理员用户名：任免按钮按「是否自己」决定是否展示
    let current_username = auth::current_user(cx).await.unwrap_or_default();
    let current_names: Vec<String> = std::iter::repeat(current_username).take(n).collect();
    // 操作成功横幅（?ok=updated 封禁解封 / ?ok=role 任免）
    let notice = super::notice(
        cx,
        &locale,
        &[
            ("updated", "admin_user_updated"),
            ("role", "admin_role_updated"),
        ],
    );
    Ok(view! {
        <div class="max-w-6xl mx-auto px-4 py-8">
            components::admin_nav::AdminNav(
                locale: loc.clone(),
                active: "users".to_string()
            )
            <h1 class="text-xl font-bold mb-6 text-foreground">
                (loader::t(&locale, "admin_users"))
            </h1>
            if let Some(ref msg) = notice {
                <p class="text-sm text-green-600 mb-4">(msg.clone())</p>
            }
            if user_list.is_empty() {
                <p class="text-muted-foreground py-16 text-center">
                    (loader::t(&locale, "no_data"))
                </p>
            } else {
                <div class="space-y-2">
                    for (user, (lc, (tok, cur))) in user_list
                        .into_iter()
                        .zip(
                            locales
                                .into_iter()
                                .zip(csrfs.into_iter().zip(current_names)),
                        ) {
                        AdminUserRow(
                            locale: lc,
                            user: user,
                            csrf: tok,
                            current_username: cur
                        )
                    }
                </div>
                components::pagination::Pagination(
                    locale: loc,
                    page_info: page_info,
                    base_url: format!("/{locale}/admin/users")
                )
            }
        </div>
    })
}

/// 单行用户：用户名、邮箱、状态、管理员标记、封禁/解封、管理员任免
#[component]
async fn AdminUserRow(locale: String, user: User, csrf: String, current_username: String) -> Result<impl View> {
    let status_key = match user.status {
        USER_STATUS_ACTIVE => "user_status_active",
        USER_STATUS_BANNED => "user_status_banned",
        _ => "user_status_pending",
    };
    let ban_url = format!("/{locale}/admin/users/{}/status", user.id);
    let role_url = format!("/{locale}/admin/users/{}/role", user.id);
    // 任免护栏：不能取消自己（自己行不展示任免按钮，端点侧同样拒绝）
    let is_self = user.username == current_username;
    let ban_label = loader::t(&locale, if user.status == USER_STATUS_BANNED {
        "admin_user_unban"
    } else {
        "admin_user_ban"
    });
    let ban_value = if user.status == USER_STATUS_BANNED {
        "active"
    } else {
        "banned"
    };
    let ban_variant = if user.status == USER_STATUS_BANNED {
        ButtonVariant::Secondary
    } else {
        ButtonVariant::Destructive
    };
    // view! 分支按 move 捕获：封禁/任免两个互斥分支各自使用独立克隆
    let ban_csrf = csrf.clone();
    let role_csrf = csrf;
    Ok(view! {
        <div
            class="bg-surface border border-border rounded-lg p-4 flex flex-wrap items-center gap-3"
        >
            <div class="min-w-0 flex-1">
                <div class="text-sm font-medium text-foreground truncate">
                    (user.username)
                </div>
                <div class="text-xs text-muted-foreground mt-0.5">(user.email)</div>
            </div>
            <div class="flex items-center gap-2">
                if user.is_admin == USER_IS_ADMIN {
                    badge(
                        variant: BadgeVariant::Primary,
                        attrs: attributes! {},
                        (loader::t(&locale, "admin_user_is_admin"))
                    )
                }
                if user.status == USER_STATUS_BANNED {
                    badge(
                        variant: BadgeVariant::Destructive,
                        attrs: attributes! {},
                        (loader::t(&locale, status_key))
                    )
                } else if user.status == USER_STATUS_ACTIVE {
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
                if user.is_admin == USER_IS_ADMIN {
                    // 管理员行：非自己可取消管理员；自己无任免按钮（护栏）
                    if !is_self {
                        <form method="POST" action=(role_url) class="inline">
                            CsrfField(token: role_csrf)
                            <button
                                type="submit"
                                name="role"
                                value="user"
                                class=(button_variants(ButtonVariant::Secondary, ButtonSize::Sm))
                            >
                                (loader::t(&locale, "admin_user_demote"))
                            </button>
                        </form>
                    }
                } else {
                    // 非管理员行：封禁/解封 + 设为管理员
                    <form method="POST" action=(ban_url) class="inline">
                        CsrfField(token: ban_csrf)
                        <button
                            type="submit"
                            name="to"
                            value=(ban_value)
                            class=(button_variants(ban_variant, ButtonSize::Sm))
                        >
                            (ban_label)
                        </button>
                    </form>
                    <form method="POST" action=(role_url) class="inline">
                        CsrfField(token: role_csrf)
                        <button
                            type="submit"
                            name="role"
                            value="admin"
                            class=(button_variants(ButtonVariant::Secondary, ButtonSize::Sm))
                        >
                            (loader::t(&locale, "admin_user_promote"))
                        </button>
                    </form>
                }
            </div>
        </div>
    })
}

#[derive(Deserialize)]
pub struct BanForm {
    pub to: String,
    #[serde(default)]
    pub csrf_token: String,
}

/// 封禁/解封：改状态；封禁时踢掉该用户全部会话
#[topcoat::router::route(POST "/{locale}/admin/users/{id}/status")]
pub async fn admin_user_status(cx: &Cx, Form(form): Form<BanForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let user_id = path_param_segment(cx, "id");
    let base = format!("/{locale}/admin/users");
    let status = if form.to == "banned" {
        USER_STATUS_BANNED
    } else if form.to == "active" {
        USER_STATUS_ACTIVE
    } else {
        return Ok(form::redirect(&base));
    };
    let Some(user) = users::find_user_by_id(user_id).await.ok().flatten() else {
        return Ok(form::redirect(&base));
    };
    // 管理员不可被封禁
    if user.is_admin == USER_IS_ADMIN {
        return Ok(form::redirect(&base));
    }
    if users::set_user_status(user_id, status).await.is_err() {
        return Ok(form::redirect(&base));
    }
    if status == USER_STATUS_BANNED {
        let _ = session::remove_all(&user.username).await;
    }
    Ok(form::redirect(&(base + "?ok=updated")))
}

#[derive(Deserialize)]
pub struct RoleForm {
    pub role: String,
    #[serde(default)]
    pub csrf_token: String,
}

/// 管理员任免：CSRF → 目标存在 → 不可取消自己 → 写 is_admin
#[topcoat::router::route(POST "/{locale}/admin/users/{id}/role")]
pub async fn admin_user_role(cx: &Cx, Form(form): Form<RoleForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let user_id = path_param_segment(cx, "id");
    let base = format!("/{locale}/admin/users");
    let flag = match form.role.as_str() {
        "admin" => USER_IS_ADMIN,
        "user" => 0,
        _ => return Ok(form::redirect(&base)),
    };
    let Some(user) = users::find_user_by_id(user_id).await.ok().flatten() else {
        return Ok(form::redirect(&base));
    };
    // 护栏：不能取消自己的管理员权限
    if flag == 0 && auth::current_user(cx).await.as_deref() == Some(user.username.as_str()) {
        return Ok(form::redirect(&base));
    }
    if users::set_user_is_admin(user_id, flag).await.is_err() {
        return Ok(form::redirect(&base));
    }
    Ok(form::redirect(&(base + "?ok=role")))
}
