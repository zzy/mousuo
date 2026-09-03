use topcoat::{
    Result,
    context::Cx,
    mail::{Mail, Mailbox, send},
};

use super::config;
use crate::i18n::loader;

/// 发送账户激活邮件
pub async fn send_activation(cx: &Cx, locale: &str, email_to: &str, token: &str) -> Result<()> {
    let cfg = config::config();
    let subject = loader::t(locale, "email_activation_subject");
    let scheme = topcoat::router::request::headers(cx)
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let body = format!(
        "{}\n{scheme}://{}/{locale}/users/activate?token={token}",
        loader::t(locale, "email_activation_body"),
        cfg.domain,
    );
    let mail = Mail::builder()
        .from(Mailbox::new(cfg.email_from.as_str()).expect("无效的发件地址"))
        .to([Mailbox::new(email_to).expect("无效的收件地址")])
        .subject(subject)
        .text(body)
        .build();
    if let Err(e) = send(cx, mail).await {
        eprintln!("激活邮件发送失败 -> {email_to}: {e:?}");
        return Err(e);
    }
    Ok(())
}

/// 发送密码重置邮件
pub async fn send_password_reset(cx: &Cx, locale: &str, email_to: &str, token: &str) -> Result<()> {
    let cfg = config::config();
    let subject = loader::t(locale, "email_reset_subject");
    let scheme = topcoat::router::request::headers(cx)
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let body = format!(
        "{}\n{scheme}://{}/{locale}/users/reset-password?token={token}",
        loader::t(locale, "email_reset_body"),
        cfg.domain,
    );
    let mail = Mail::builder()
        .from(Mailbox::new(cfg.email_from.as_str()).expect("无效的发件地址"))
        .to([Mailbox::new(email_to).expect("无效的收件地址")])
        .subject(subject)
        .text(body)
        .build();
    if let Err(e) = send(cx, mail).await {
        eprintln!("密码重置邮件发送失败 -> {email_to}: {e:?}");
        return Err(e);
    }
    Ok(())
}
