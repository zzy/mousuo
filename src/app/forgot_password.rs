#![allow(non_snake_case)]

use crate::common::{auth, captcha, email, form, session};
use crate::components::button::{ButtonVariant, button};
use crate::components::card::card;
use crate::components::csrf::CsrfField;
use crate::components::input::input;
use crate::components::label::label;
use crate::db::users;
use crate::i18n::loader;
use base64::Engine;
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{content::Form, error::forbidden, page, path_param_segment, response::Response},
    view::{attributes, view},
};

/// 找回密码页：账户 + 验证码；无论账户是否存在都提示「若存在则已发送」（防枚举）
#[page("/{locale}/users/forgot-password")]
pub async fn forgot_password_page(cx: &Cx) -> Result {
    let locale = path_param_segment(cx, "locale");
    let captcha = captcha::generate();
    let _ = topcoat::session::start(cx).await;
    let _ = captcha::save_answer(cx, captcha.answer).await;
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    let captcha_svg = base64::engine::general_purpose::STANDARD.encode(&captcha.svg);
    let captcha_src = format!("data:image/svg+xml;base64,{captcha_svg}");
    let sent = form::query_param(cx, "sent").is_some();
    let error = form::query_param(cx, "error").is_some();
    view! {
        <main class="max-w-md mx-auto px-4 py-8">
            card(
                attrs: attributes! { class="p-6" },
                <h1 class="text-xl font-bold text-foreground mb-6 text-center">
                    (loader::t(&locale, "forgot_password_title"))
                </h1>
                if sent {
                    <p class="text-green-600 text-sm mb-4 text-center">
                        (loader::t(&locale, "forgot_password_sent"))
                    </p>
                }
                if error {
                    <p class="text-red-500 text-sm mb-4 text-center">
                        (loader::t(&locale, "forgot_password_send_failed"))
                    </p>
                }
                <form method="POST" action="" class="space-y-4">
                    CsrfField(token: csrf)
                    <div class="space-y-1">
                        label(
                            attrs: attributes! {},
                            (loader::t(&locale, "sign_in_account"))
                        )
                        input(
                            attrs: attributes! { type="text" name="account" required="" }
                        )
                    </div>
                    <div class="space-y-3 border-t border-border pt-4 mt-4">
                        label(
                            attrs: attributes! {},
                            (loader::t(&locale, "captcha_label"))
                        )
                        <div class="flex items-center justify-center gap-2">
                            <img
                                src=(captcha_src)
                                alt=""
                                class="rounded overflow-hidden cursor-pointer shrink-0"
                                style="width:160px;height:40px;border:1px solid #d1d5db"
                                onclick="location.reload()"
                            >
                            input(
                                attrs: attributes! {
                                    type="text"
                                    name="captcha_answer"
                                    required=""
                                    placeholder=(loader::t(&locale, "captcha_placeholder"))
                                    class="w-16 text-center text-xl"
                                }
                            )
                        </div>
                    </div>
                    button(
                        variant: ButtonVariant::Primary,
                        attrs: attributes! { type="submit" class="w-full justify-center" },
                        (loader::t(&locale, "forgot_password_submit"))
                    )
                </form>
            )
        </main>
    }
}

#[derive(Deserialize)]
pub struct ForgotForm {
    pub account: String,
    pub captcha_answer: String,
    #[serde(default)]
    pub csrf_token: String,
}

/// 发起找回：生成重置 token + 发信；账户不存在也返回同样提示（防枚举）
#[topcoat::router::route(POST "/{locale}/users/forgot-password")]
pub async fn forgot_password_action(cx: &Cx, Form(form): Form<ForgotForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    let base = format!("/{locale}/users/forgot-password");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    if !captcha::verify(cx, &form.captcha_answer).await {
        return Ok(form::redirect(&(base.clone() + "?error=1")));
    }
    match users::issue_password_reset(&form.account).await {
        Ok((email_to, token)) => {
            if email::send_password_reset(cx, &locale, &email_to, &token)
                .await
                .is_err()
            {
                return Ok(form::redirect(&(base.clone() + "?error=1")));
            }
            Ok(form::redirect(&(base + "?sent=1")))
        }
        // 账户不存在：与成功相同提示，不泄露注册状态
        Err(_) => Ok(form::redirect(&(base + "?sent=1"))),
    }
}

/// 重置密码页：token 表单（新密码 ×2）
#[page("/{locale}/users/reset-password")]
pub async fn reset_password_page(cx: &Cx) -> Result {
    let locale = path_param_segment(cx, "locale");
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    let token = form::query_param(cx, "token").unwrap_or_default();
    let error = form::error_message(cx, &locale, &["password_weak", "password_mismatch", "token_invalid"]);
    let invalid = form::query_param(cx, "invalid").is_some();
    view! {
        <main class="max-w-md mx-auto px-4 py-8">
            card(
                attrs: attributes! { class="p-6" },
                <h1 class="text-xl font-bold text-foreground mb-6 text-center">
                    (loader::t(&locale, "reset_password_title"))
                </h1>
                if invalid {
                    <p class="text-red-500 text-sm mb-4 text-center">
                        (loader::t(&locale, "reset_password_invalid"))
                    </p>
                }
                if let Some(ref msg) = error {
                    <p class="text-red-500 text-sm mb-4 text-center">(msg.clone())</p>
                }
                <form method="POST" action="" class="space-y-4">
                    CsrfField(token: csrf)
                    <input type="hidden" name="token" value=(token)>
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
                            (loader::t(
                                &locale,
                                "register_confirm_password",
                            ))
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
                        (loader::t(&locale, "reset_password_submit"))
                    )
                </form>
            )
        </main>
    }
}

#[derive(Deserialize)]
pub struct ResetForm {
    pub token: String,
    pub new_password: String,
    pub confirm_password: String,
    #[serde(default)]
    pub csrf_token: String,
}

/// 完成重置：token 校验 → 更新 cred → 清 token → 踢全部会话
#[topcoat::router::route(POST "/{locale}/users/reset-password")]
pub async fn reset_password_action(cx: &Cx, Form(form): Form<ResetForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    let base = format!("/{locale}/users/reset-password?token={}", form.token);
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    if form.new_password != form.confirm_password {
        return Ok(form::redirect(&(base.clone() + "&error=password_mismatch")));
    }
    if !auth::password_strong(&form.new_password) {
        return Ok(form::redirect(&(base.clone() + "&error=password_weak")));
    }
    let cred = auth::hash_credential(&form.new_password);
    match users::complete_password_reset(&form.token, &cred).await {
        Ok(username) => {
            let _ = session::remove_all(&username).await;
            Ok(form::redirect(&format!("/{locale}/sign-in?notice=reset")))
        }
        Err(_) => Ok(form::redirect(&(base + "&invalid=1"))),
    }
}
