#![allow(non_snake_case)]

use crate::common::{form, media, session};
use crate::components;
use crate::components::button::{ButtonVariant, button};
use crate::components::card::card;
use crate::components::csrf::CsrfField;
use crate::i18n::loader;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::multipart::Multipart,
        error::forbidden,
        page,
        path_param_segment,
        response::Response,
    },
    view::{attributes, view},
};

/// 上传页：表单 + 结果展示（?ok=image|video&url=/media/…）
#[page("/{locale}/admin/upload")]
pub async fn admin_upload_page(cx: &Cx) -> Result {
    let locale = path_param_segment(cx, "locale");
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    let url = form::query_param(cx, "url");
    let ok = form::query_param(cx, "ok");
    let is_video = ok.as_deref() == Some("video");
    let error_message = form::error_message(
        cx,
        &locale,
        &["upload_empty", "upload_too_large", "upload_type", "upload_failed"],
    );
    view! {
        <div class="max-w-2xl mx-auto px-4 py-8">
            components::admin_nav::AdminNav(
                locale: locale.to_string(),
                active: "upload".to_string()
            )
            <h1 class="text-xl font-bold mb-6 text-foreground">
                (loader::t(&locale, "admin_upload"))
            </h1>
            if let Some(ref msg) = error_message {
                <p class="text-red-500 text-sm mb-4">(msg.clone())</p>
            }
            card(
                attrs: attributes! { class="p-6" },
                <form
                    action=""
                    method="post"
                    enctype="multipart/form-data"
                    class="space-y-4"
                >
                    CsrfField(token: csrf)
                    <div class="space-y-1">
                        <input
                            type="file"
                            name="file"
                            accept="image/*,video/*"
                            required=""
                            class="w-full text-sm file:mr-3 file:py-1.5 file:px-3 file:rounded file:border-0 file:bg-blue-50 file:text-blue-700"
                        >
                    </div>
                    <p class="text-xs text-muted-foreground">
                        (loader::t(&locale, "admin_upload_hint"))
                    </p>
                    button(
                        variant: ButtonVariant::Primary,
                        attrs: attributes! { type="submit" class="w-full justify-center" },
                        (loader::t(&locale, "admin_upload_submit"))
                    )
                </form>
            )
            if let Some(ref u) = url {
                <div class="mt-6 space-y-3">
                    <h2 class="text-sm font-semibold text-foreground">
                        (loader::t(&locale, "admin_upload_media_url"))
                    </h2>
                    <div class="bg-surface border border-border rounded-lg p-4">
                        <code class="text-xs text-foreground break-all">
                            (u.clone())
                        </code>
                    </div>
                    if is_video {
                        crate::components::hls_player::HlsPlayer(src: u.clone())
                        <p class="text-xs text-muted-foreground">
                            (loader::t(&locale, "admin_upload_video_hint"))
                        </p>
                    } else {
                        <img
                            src=(u.clone())
                            alt=""
                            class="max-w-full h-auto rounded-lg bg-white"
                        >
                    }
                </div>
            }
        </div>
    }
}

/// 上传处理：图片直存；视频 ffprobe 校验 + HLS 转码；303 回结果页（PRG）
#[topcoat::router::route(POST "/{locale}/admin/upload")]
pub async fn admin_upload_handler(cx: &Cx, mut form_data: Multipart) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    let base = format!("/{locale}/admin/upload");
    let mut file_data: Option<Vec<u8>> = None;
    let mut mime = String::new();
    let mut csrf = String::new();
    while let Some(field) = form_data
        .next_field()
        .await
        .map_err(|e| topcoat::router::error::bad_request(e.to_string()))?
    {
        match field.name().unwrap_or("") {
            "file" => {
                mime = field.content_type().unwrap_or("").to_string();
                file_data = Some(field.bytes().await.unwrap_or_default().to_vec());
            }
            "csrf_token" => csrf = field.text().await.unwrap_or_default(),
            _ => {}
        }
    }
    // CSRF 校验必须在任何副作用（文件写入）之前
    if !session::verify_csrf(cx, &csrf).await {
        return Err(forbidden().into());
    }
    let Some(bytes) = file_data.filter(|b| !b.is_empty()) else {
        return Ok(form::redirect(&(base + "?error=upload_empty")));
    };
    if bytes.len() > media::UPLOAD_MAX_BYTES {
        return Ok(form::redirect(&(base + "?error=upload_too_large")));
    }
    let Some(ext) = media::mime_to_extension(&mime) else {
        return Ok(form::redirect(&(base + "?error=upload_type")));
    };
    let result = if media::is_image_ext(ext) {
        media::save_image(&bytes, ext).await
    } else {
        media::save_video(&bytes, ext).await
    };
    match result {
        Ok(url) => {
            let kind = if media::is_image_ext(ext) { "image" } else { "video" };
            Ok(form::redirect(&format!("{base}?ok={kind}&url={url}")))
        }
        Err(e) => {
            eprintln!("媒体上传失败: {e}");
            Ok(form::redirect(&(base + "?error=upload_failed")))
        }
    }
}
