#![allow(non_snake_case)]

use crate::common::{auth, form, session};
use crate::components::button::{ButtonVariant, button};
use crate::components::card::card;
use crate::components::csrf::CsrfField;
use crate::components::input::input;
use crate::components::label::label;
use crate::db::users;
use crate::i18n::loader;
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{content::Form, error::forbidden, page, path_param_segment, response::Response},
    view::{View, attributes, view},
};

/// 修改密码页（需登录；未登录 302 签入）
#[page("/{locale}/account/password")]
pub async fn change_password_page(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    // 未登录：渲染提示页（页面处理器不可重定向，守卫语义交给 LoginGuard 同类层之外处理）
    let signed_in = auth::current_user(cx).await.is_some();
    let ok = form::query_param(cx, "ok").is_some();
    let error = form::error_message(cx, &locale, &["password_weak", "old_incorrect", "password_mismatch"]);
    Ok(view! {
        if signed_in {
            <main class="max-w-md mx-auto px-4 py-8">
                card(
                    attrs: attributes! { class="p-6" },
                    <h1 class="text-xl font-bold text-foreground mb-6 text-center">
                        (loader::t(&locale, "account_change_password"))
                    </h1>
                    if ok {
                        <p class="text-green-600 text-sm mb-4 text-center">
                            (loader::t(&locale, "account_password_changed"))
                        </p>
                    }
                    if let Some(ref msg) = error {
                        <p class="text-red-500 text-sm mb-4 text-center">
                            (msg.clone())
                        </p>
                    }
                    <form method="POST" action="" class="space-y-4">
                        CsrfField(token: csrf)
                        <div class="space-y-1">
                            label(
                                attrs: attributes! {},
                                (loader::t(&locale, "account_old_password"))
                            )
                            input(
                                attrs: attributes! {
                                    type="password"
                                    name="old_password"
                                    required=""
                                    autocomplete="current-password"
                                }
                            )
                        </div>
                        <div class="space-y-1">
                            label(
                                attrs: attributes! {},
                                (loader::t(&locale, "register_password"))
                            )
                            input(
                                attrs: attributes! {
                                    type="password"
                                    name="new_password"
                                    required=""
                                    autocomplete="new-password"
                                }
                            )
                        </div>
                        <div class="space-y-1">
                            label(
                                attrs: attributes! {},
                                (loader::t(&locale, "register_confirm_password"))
                            )
                            input(
                                attrs: attributes! {
                                    type="password"
                                    name="confirm_password"
                                    required=""
                                    autocomplete="new-password"
                                }
                            )
                        </div>
                        button(
                            variant: ButtonVariant::Primary,
                            attrs: attributes! { type="submit" class="w-full justify-center" },
                            (loader::t(&locale, "account_save_password"))
                        )
                    </form>
                )
            </main>
        } else {
            <main class="max-w-md mx-auto px-4 py-16 text-center">
                <p class="text-muted-foreground mb-4">
                    (loader::t(&locale, "account_requires_sign_in"))
                </p>
                <a
                    href=(format!("/{locale}/sign-in?next=/{locale}/account/password"))
                    class="text-blue-600 dark:text-blue-400 hover:underline"
                >
                    (loader::t(&locale, "sign_in"))
                </a>
            </main>
        }
    })
}

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    pub old_password: String,
    pub new_password: String,
    pub confirm_password: String,
    #[serde(default)]
    pub csrf_token: String,
}

/// 修改密码：CSRF → 验旧密码 → 强度校验 → 更新 cred → 踢其它会话（保留当前）
#[topcoat::router::route(POST "/{locale}/account/password")]
pub async fn change_password_action(cx: &Cx, Form(form): Form<ChangePasswordForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    let base = format!("/{locale}/account/password");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let Some(username) = auth::current_user(cx).await else {
        return Ok(form::redirect(&format!("/{locale}/sign-in?next={base}")));
    };
    if form.new_password != form.confirm_password {
        return Ok(form::redirect(&(base.clone() + "?error=password_mismatch")));
    }
    if !auth::password_strong(&form.new_password) {
        return Ok(form::redirect(&(base.clone() + "?error=password_weak")));
    }
    let Some(user) = users::find_user(&username).await.ok().flatten() else {
        return Ok(form::redirect(&(base.clone() + "?error=old_incorrect")));
    };
    if !auth::verify_credential(&form.old_password, &user.cred) {
        return Ok(form::redirect(&(base.clone() + "?error=old_incorrect")));
    }
    if users::update_cred(&username, &auth::hash_credential(&form.new_password))
        .await
        .is_err()
    {
        return Ok(form::redirect(&(base.clone() + "?error=old_incorrect")));
    }
    // 踢掉其它会话，保留当前
    if let Some(hash) = topcoat::session::token_hash(cx).await.ok().flatten() {
        let _ = session::remove_all_except(&hash, &username).await;
    }
    Ok(form::redirect(&(base + "?ok=1")))
}
