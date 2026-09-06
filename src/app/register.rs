#![allow(non_snake_case)]

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;

use crate::common::auth;
use crate::common::captcha;
use crate::common::email;
use crate::common::form;
use crate::common::markdown;
use crate::common::session;
use crate::components;
use crate::components::auth_dialog::{AuthDialogWidth, auth_dialog};
use crate::components::button::{ButtonSize, ButtonVariant, button, button_variants};
use crate::components::captcha_area::CaptchaArea;
use crate::components::card::card;
use crate::components::csrf::CsrfField;
use crate::components::input::input;
use crate::components::label::label;
use crate::db::users;
use crate::i18n::loader;
use crate::common::icons;
use topcoat::{
    Result,
    context::Cx,
    icon::icon,
    router::{
        content::Form, error::{bad_request, forbidden}, path_param_segment, query_params, response::Response,
    },
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

#[derive(Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
    pub introduction: String,
    pub captcha_answer: String,
    #[serde(default)]
    pub csrf_token: String,
}

#[topcoat::router::page("/{locale}/register")]
pub async fn register_page(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    // 先签发 session（否则验证码/CSRF 无处存放）
    let _ = topcoat::session::start(cx).await;
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    let error_message = form::error_message(
        cx,
        &locale,
        &[
            "captcha",
            "password_weak",
            "password_mismatch",
            "exist",
        ],
    );
    let success_username =
        form::query_param(cx, "username").filter(|_| form::query_param(cx, "success").is_some());
    let mail_failed = form::query_param(cx, "mail").is_some();
    Ok(view! {
        RegisterCard(
            locale: locale.to_string(),
            error_message: error_message,
            success_username: success_username,
            mail_failed: mail_failed,
            csrf: csrf
        )
    })
}

#[topcoat::router::route(POST "/{locale}/register")]
pub async fn register_action(cx: &Cx, Form(form): Form<RegisterForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let error_url = format!("/{locale}/register");
    if !captcha::verify(cx, &form.captcha_answer).await {
        return Ok(form::redirect(&(error_url + "?error=captcha")));
    }
    if form.password != form.confirm_password {
        return Ok(form::redirect(&(error_url + "?error=password_mismatch")));
    }
    if !auth::password_strong(&form.password) {
        return Ok(form::redirect(&(error_url + "?error=password_weak")));
    }
    if users::find_user(&form.username)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return Ok(form::redirect(&(error_url + "?error=exist")));
    }
    let (found_user, activation_token) = users::register_user(
        &form.username,
        &form.password,
        &form.email,
        &form.introduction,
    )
    .await
    .map_err(|e| bad_request(e))?;
    // 发送激活邮件；失败在成功页提示，不静默吞掉
    let mail_failed = email::send_activation(cx, &locale, &found_user.email, &activation_token)
        .await
        .is_err();
    Ok(form::redirect(&format!(
        "/{locale}/register?success=1&username={}{}",
        found_user.username,
        if mail_failed { "&mail=1" } else { "" }
    )))
}

#[derive(Deserialize)]
struct MdForm {
    md: String,
    #[serde(default)]
    csrf_token: String,
}

/// Markdown 预览端点
#[topcoat::router::route(POST "/{locale}/md-preview")]
pub async fn md_preview(cx: &Cx, Form(form): Form<MdForm>) -> Result<Response> {
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let html = markdown::render_md(&form.md);
    Ok(Response::builder()
        .header("content-type", "text/html; charset=utf-8")
        .body(topcoat::router::Body::from(html))
        .unwrap())
}

#[query_params]
struct ActivateQuery {
    token: String,
}

/// 账户激活页
#[topcoat::router::page("/{locale}/users/activate")]
pub async fn activate_page(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let params = query_params::<ActivateQuery>(cx).ok();
    let (success, message) = match params
        .as_ref()
        .and_then(|p| if p.token.is_empty() { None } else { Some(&p.token) })
    {
        Some(token) => match users::activate_by_token(token).await {
            Ok(()) => (true, loader::t(&locale, "user_activated").to_string()),
            Err(e) => (false, e),
        },
        None => (false, loader::t(&locale, "page_error_404").to_string()),
    };
    Ok(view! {
        <main class="min-h-[80vh] flex items-center justify-center px-4">
            card(
                attrs: attributes! { class="p-8 text-center max-w-md" },
                if success {
                    <h1 class="text-xl font-bold text-green-600 mb-4">
                        (loader::t(&locale, "user_activated"))
                    </h1>
                    <p class="text-foreground mb-4">(message)</p>
                    <a
                        href=(format!("/{locale}/sign-in"))
                        class=(button_variants(ButtonVariant::Primary, ButtonSize::Md))
                    >
                        (loader::t(&locale, "sign_in"))
                    </a>
                } else {
                    <h1 class="text-xl font-bold text-red-500 mb-4">
                        (loader::t(&locale, "user_activate_problem"))
                    </h1>
                    <p class="text-muted-foreground mb-4">(message)</p>
                }
            )
        </main>
    })
}

/// 激活邮件重发页（带验证码）；account 由签入页 not_activation 入口带入
#[topcoat::router::page("/{locale}/users/resend")]
pub async fn resend_page(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let captcha = captcha::generate();
    // 先签发 session（否则验证码无处存放），再将答案写入 session
    let _ = topcoat::session::start(cx).await;
    let _ = captcha::save_answer(cx, captcha.answer).await;
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    let captcha_svg = STANDARD.encode(&captcha.svg);
    let captcha_src = format!("data:image/svg+xml;base64,{captcha_svg}");
    let account = form::query_param(cx, "account").unwrap_or_default();
    let sent = form::query_param(cx, "sent").is_some();
    let send_error = form::query_param(cx, "error").is_some();
    Ok(view! {
        <main class="max-w-md mx-auto px-4 py-8">
            card(
                attrs: attributes! { class="p-6" },
                <h1 class="text-xl font-bold text-foreground mb-6 text-center">
                    (loader::t(&locale, "resend_title"))
                </h1>
                if sent {
                    <p class="text-green-600 text-sm mb-4 text-center">
                        (loader::t(&locale, "resend_sent"))
                    </p>
                }
                if send_error {
                    <p class="text-red-500 text-sm mb-4 text-center">
                        (loader::t(&locale, "resend_failed"))
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
                            attrs: attributes! { type="text" name="account" required="" value=(account) }
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
                        (loader::t(&locale, "resend_submit"))
                    )
                </form>
            )
        </main>
    })
}

#[derive(Deserialize)]
struct ResendForm {
    pub account: String,
    pub captcha_answer: String,
    #[serde(default)]
    pub csrf_token: String,
}

/// 重发激活邮件：CSRF + 验证码 → 轮换 token → 发信
#[topcoat::router::route(POST "/{locale}/users/resend")]
pub async fn resend_action(cx: &Cx, Form(form): Form<ResendForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    let base = format!("/{locale}/users/resend?account={}", form.account);
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    if !captcha::verify(cx, &form.captcha_answer).await {
        return Ok(form::redirect(&(base.clone() + "&error=1")));
    }
    match users::rotate_activation_token(&form.account).await {
        Ok((email_to, token)) => {
            if email::send_activation(cx, &locale, &email_to, &token)
                .await
                .is_err()
            {
                return Ok(form::redirect(&(base + "&error=1")));
            }
            Ok(form::redirect(&(base + "&sent=1")))
        }
        Err(_) => Ok(form::redirect(&(base + "&error=1"))),
    }
}

#[component]
async fn RegisterCard(
    cx: &Cx,
    locale: String,
    error_message: Option<String>,
    success_username: Option<String>,
    mail_failed: bool,
    csrf: String,
) -> Result<impl View> {
    let intro_default = "## About Me".to_string();
    let cap_locale = locale.clone();
    let captcha_nonce = signal(cx, || 0.0);
    Ok(view! {
        auth_dialog(
            locale: locale.clone(),
            width: AuthDialogWidth::Register,
            <div>
                <h1 class="text-xl font-bold text-foreground mb-6 text-center">
                    (loader::t(&locale, "register"))
                </h1>
                // 表单始终渲染，成功时模糊 + 禁止交互
                <div
                    style=(if success_username.is_some() {
                        "opacity:0.35;filter:blur(4px);pointer-events:none;transition:all 0.3s"
                    } else {
                        "opacity:1;filter:none;pointer-events:auto;transition:all 0.3s"
                    })
                >
                    <form
                        method="POST"
                        action=""
                        onsubmit=(format!(
                            "var b=this.querySelector('button[type=submit]');if(b){{b.disabled=true;b.textContent='{}'}}",
                            loader::t(&locale, "submitting"),
                        ))
                    >
                        <input type="hidden" name="lang" value=(locale.as_str())>
                        CsrfField(token: csrf.clone())
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div class="space-y-1">
                                label(
                                    attrs: attributes! {},
                                    (loader::t(&locale, "register_username"))
                                    " *"
                                )
                                input(
                                    attrs: attributes! {
                                        type="text"
                                        name="username"
                                        required=""
                                        pattern="[a-z0-9_-]+"
                                        autocomplete="username"
                                    }
                                )
                            </div>
                            <div class="space-y-1">
                                label(
                                    attrs: attributes! {},
                                    (loader::t(&locale, "register_email"))
                                    " *"
                                )
                                input(
                                    attrs: attributes! {
                                        type="email"
                                        name="email"
                                        required=""
                                        autocomplete="email"
                                    }
                                )
                            </div>
                            <div class="space-y-1">
                                label(
                                    attrs: attributes! {},
                                    (loader::t(&locale, "register_password"))
                                    " *"
                                )
                                input(
                                    attrs: attributes! {
                                        type="password"
                                        name="password"
                                        required=""
                                        autocomplete="new-password"
                                    }
                                )
                            </div>
                            <div class="space-y-1">
                                label(
                                    attrs: attributes! {},
                                    (loader::t(&locale, "register_confirm_password"))
                                    " *"
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
                        </div>
                        <div class="space-y-4 mt-4">
                            <div>
                                label(
                                    attrs: attributes! {},
                                    (loader::t(&locale, "register_intro"))
                                )
                                components::markdown_editor::MarkdownEditor(
                                    locale: locale.clone(),
                                    name: "introduction".to_string(),
                                    rows: 8,
                                    value: intro_default,
                                    required: true,
                                    csrf: csrf.clone()
                                )
                            </div>
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
                            (loader::t(&locale, "register"))
                        )
                        if let Some(ref msg) = error_message {
                            <p class="text-red-500 text-sm text-center">
                                (msg.clone())
                            </p>
                        }
                    </form>
                </div>
                <p class="mt-4 text-sm text-center text-muted-foreground">
                    (loader::t(&locale, "register_have_account"))
                    " "
                    <a
                        href=(format!("/{locale}/sign-in"))
                        class="text-blue-500 hover:underline"
                    >
                        (loader::t(&locale, "register_go_sign_in"))
                    </a>
                </p>
                if let Some(ref name) = success_username {
                    <div class="modal-overlay">
                        <div class="modal-card">
                            <div class="modal-icon">
                                icon(data: icons::CHECK, size: 30)
                            </div>
                            <p class="modal-text">
                                (loader::t(&locale, "register_success"))
                                " "
                                (name.clone())
                                "！"
                            </p>
                            if mail_failed {
                                <p class="text-amber-600 text-sm mb-4 text-center">
                                    (loader::t(&locale, "register_mail_failed"))
                                </p>
                            }
                            <div class="modal-actions">
                                <a
                                    href=(format!("/{locale}/sign-in"))
                                    class=(button_variants(
                                        ButtonVariant::Primary,
                                        ButtonSize::Md,
                                    ))
                                >
                                    (loader::t(&locale, "register_go_sign_in"))
                                </a>
                                <a
                                    href=(format!("/{locale}"))
                                    class=(button_variants(
                                        ButtonVariant::Primary,
                                        ButtonSize::Md,
                                    ))
                                >
                                    (loader::t(&locale, "go_home"))
                                </a>
                            </div>
                        </div>
                    </div>
                }
            </div>
        )
    })
}
