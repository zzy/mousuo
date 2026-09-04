#![allow(non_snake_case)]

use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::card;
use crate::i18n::loader;
use topcoat::{
    Result,
    view::{View, component, view, attributes},
};

/// Markdown 编辑器组件 — 带预览切换的 textarea
/// 预览通过服务端渲染（对齐 ftbsite PreviewMd），隐藏域提交 markdown 源码
#[component]
pub async fn MarkdownEditor(
    locale: String,
    name: String,
    rows: u32,
    value: String,
    required: bool,
    csrf: String,
) -> Result<impl View> {
    let editor_id = format!("md-ed-{name}");
    let preview_id = format!("md-pv-{name}");
    let req = if required { "required" } else { "" };
    Ok(view! {
        <div class="md-editor">
            <div class="flex items-center gap-2 mb-2">
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        type="button"
                        class="border-0 cursor-pointer"
                        id=(format!("md-btn-pv-{name}"))
                        onclick=(format!(
                            "mdPreview('{editor_id}','{preview_id}','{locale}')",
                        ))
                    },
                    (loader::t(&locale, "md_preview"))
                )
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        type="button"
                        class="border-0 cursor-pointer hidden"
                        id=(format!("md-btn-src-{name}"))
                        onclick=(format!("mdSource('{editor_id}','{preview_id}')"))
                    },
                    (loader::t(&locale, "md_source"))
                )
                <div class="flex-1"></div>
            </div>
            <textarea
                id=(editor_id)
                name=(name.clone())
                rows=(rows)
                class="form-input"
                required=(req)
            >
                (value)
            </textarea>
            <div id=(preview_id) class="hidden">
                card(attrs: attributes! { class="p-4 prose-sm" })
            </div>
        </div>
        <script>
            (format!(
                "async function mdPreview(ed,pv,loc){{var t=document.getElementById(ed);var p=document.getElementById(pv);var r=await fetch('/'+loc+'/md-preview',{{method:'POST',headers:{{'Content-Type':'application/x-www-form-urlencoded'}},body:'md='+encodeURIComponent(t.value)+'&csrf_token='+encodeURIComponent('{csrf}')}});var h=await r.text();p.innerHTML=h;t.classList.add('hidden');p.classList.remove('hidden');document.getElementById('md-btn-pv-{name}').classList.add('hidden');document.getElementById('md-btn-src-{name}').classList.remove('hidden')}}function mdSource(ed,pv){{document.getElementById(ed).classList.remove('hidden');document.getElementById(pv).classList.add('hidden');document.getElementById('md-btn-pv-{name}').classList.remove('hidden');document.getElementById('md-btn-src-{name}').classList.add('hidden')}}",
            ))
        </script>
    })
}
