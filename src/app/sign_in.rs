#![allow(non_snake_case)]

use serde::Deserialize;

use crate::common::auth;
use crate::common::form;
use crate::common::icons;
use crate::common::session;
use crate::components::auth_dialog::{AuthDialogWidth, auth_dialog};
use crate::components::button::{ButtonVariant, button};
use crate::components::captcha_area::CaptchaArea;
use crate::components::csrf::CsrfField;
use crate::components::input::input;
use crate::components::label::label;
use crate::db::users;
use crate::i18n::loader;
use topcoat::{
    Result,
    context::Cx,
    icon::icon,
    router::{
        content::Form, error::{bad_request, forbidden}, path_param_segment, query_params,
        response::Response,
    },
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

#[derive(Deserialize)]
pub struct SignInForm {
    pub account: String,
    pub password: String,
    pub captcha_answer: String,
    /// 登录成功后的回跳路径（可选，校验见 safe_next）
    #[serde(default)]
    pub next: String,
    #[serde(default)]
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct SignOutForm {
    #[serde(default)]
    pub csrf_token: String,
}

#[query_params]
pub struct SignInQuery {
    pub next: Option<String>,
}

#[topcoat::router::page("/{locale}/sign-in")]
pub async fn sign_in_page(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    // 先签发 session（否则验证码/CSRF 无处存放）
    let _ = topcoat::session::start(cx).await;
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    let error_message = form::error_message(
        cx,
        &locale,
        &[
            "captcha",
            "incorrect",
            "not_activation",
            "banned",
            "security",
        ],
    );
    let next = query_params::<SignInQuery>(cx)
        .ok()
        .and_then(|p| p.next.clone())
        .and_then(|n| form::safe_next(&n));
    let resend_account = form::query_param(cx, "account")
        .filter(|_| form::query_param(cx, "error").as_deref() == Some("not_activation"));
    Ok(view! {
        SignInCard(
            locale: locale.to_string(),
            error_message: error_message,
            next: next.unwrap_or_default(),
            resend_account: resend_account,
            csrf: csrf
        )
    })
}

#[topcoat::router::route(POST "/{locale}/sign-in")]
pub async fn sign_in_action(cx: &Cx, Form(form): Form<SignInForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let error_url = format!("/{locale}/sign-in");
    if !crate::common::captcha::verify(cx, &form.captcha_answer).await {
        return Ok(form::redirect(&(error_url + "?error=captcha")));
    }
    let found_user = match users::find_by_account(&form.account).await {
        Ok(Some(user)) => user,
        _ => return Ok(form::redirect(&(error_url + "?error=incorrect"))),
    };
    // 检查账户是否已激活
    if found_user.status == crate::common::constant::USER_STATUS_PENDING {
        return Ok(form::redirect(&format!(
            "{error_url}?error=not_activation&account={}",
            form.account
        )));
    }
    // 封禁账户拒绝登录
    if found_user.status == crate::common::constant::USER_STATUS_BANNED {
        return Ok(form::redirect(&(error_url + "?error=banned")));
    }
    if !auth::verify_credential(&form.password, &found_user.cred) {
        return Ok(form::redirect(&(error_url + "?error=incorrect")));
    }
    auth::sign_in(cx, &found_user.username)
        .await
        .map_err(|e| bad_request(e))?;
    // 登录成功：回跳 next（已校验的相对路径）或语言首页
    if let Some(target) = form::safe_next(&form.next) {
        return Ok(form::redirect(&target));
    }
    Ok(form::redirect(&format!("/{locale}")))
}

#[topcoat::router::route(POST "/{locale}/sign-out")]
pub async fn sign_out_action(cx: &Cx, Form(form): Form<SignOutForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let _ = auth::sign_out(cx).await;
    Ok(form::redirect(&format!("/{locale}")))
}

#[component]
async fn SignInCard(
    cx: &Cx,
    locale: String,
    error_message: Option<String>,
    next: String,
    resend_account: Option<String>,
    csrf: String,
) -> Result<impl View> {
    let cap_locale = locale.clone();
    let captcha_nonce = signal(cx, || 0.0);
    Ok(view! {
        auth_dialog(
            locale: locale.clone(),
            width: AuthDialogWidth::SignIn,
            <div class="space-y-4">
                <h1 class="text-xl font-bold text-foreground mb-6 text-center">
                    (loader::t(&locale, "sign_in"))
                </h1>
                <form
                    method="POST"
                    action=""
                    class="space-y-4"
                    onsubmit=(format!(
                        "var b=this.querySelector('button[type=submit]');if(b){{b.disabled=true;b.textContent='{}'}}",
                        loader::t(&locale, "signing_in"),
                    ))
                >
                    <input type="hidden" name="next" value=(next)>
                    CsrfField(token: csrf)
                    <div class="space-y-1">
                        label(
                            attrs: attributes! {},
                            (loader::t(&locale, "sign_in_account"))
                        )
                        input(
                            attrs: attributes! {
                                type="text"
                                name="account"
                                required=""
                                autocomplete="username"
                            }
                        )
                    </div>
                    <div class="space-y-1">
                        label(
                            attrs: attributes! {},
                            (loader::t(&locale, "sign_in_password"))
                        )
                        input(
                            attrs: attributes! {
                                type="password"
                                name="password"
                                required=""
                                autocomplete="current-password"
                            }
                        )
                    </div>
                    <div class="space-y-3 border-t border-border pt-4 mt-4">
                        label(
                            attrs: attributes! {},
                            (loader::t(&locale, "captcha_label"))
                        )
                        <div class="flex items-center gap-2">
                            CaptchaArea(
                                locale: $(cap_locale),
                                nonce: $(captcha_nonce.get())
                            )
                            <button
                                type="button"
                                @click=$(|_e: Event| {
                                    captcha_nonce.set(captcha_nonce.get() + 1.0)
                                })
                                onclick="document.getElementById('auth-submit').disabled=true"
                                class="text-blue-500 hover:text-blue-700 text-lg font-bold shrink-0 leading-none border-0 bg-transparent cursor-pointer"
                            >
                                icon(data: icons::REFRESH)
                            </button>
                        </div>
                    </div>
                    button(
                        variant: ButtonVariant::Primary,
                        attrs: attributes! {
                            type="submit"
                            id="auth-submit"
                            disabled=""
                            class="w-full justify-center mt-4"
                        },
                        (loader::t(&locale, "sign_in"))
                    )
                    if let Some(ref msg) = error_message {
                        <p class="text-red-500 text-sm text-center">(msg.clone())</p>
                    }
                    if let Some(ref acc) = resend_account {
                        <p class="text-sm text-center">
                            <a
                                href=(format!(
                                    "/{locale}/users/resend?account={}",
                                    acc.clone(),
                                ))
                                class="text-blue-600 dark:text-blue-400 hover:underline no-underline"
                            >
                                (loader::t(&locale, "resend_link"))
                            </a>
                        </p>
                    }
                </form>
                <p class="mt-4 text-sm text-center text-muted-foreground">
                    (loader::t(&locale, "sign_in_new_user"))
                    " "
                    <button
                        class="text-blue-500 hover:underline border-0 bg-transparent cursor-pointer"
                        onclick=(format!("location='/{}/register'", locale))
                    >
                        (loader::t(&locale, "sign_in_create_account"))
                    </button>
                </p>
            </div>
        )
    })
}
