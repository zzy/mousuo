#![allow(non_snake_case)]

use base64::{Engine, engine::general_purpose::STANDARD};

use crate::common::captcha;
use crate::components::input::input;
use crate::i18n::loader;
use topcoat::{
    Result,
    context::Cx,
    runtime::shard,
    view::{View, attributes, view},
};

/// 验证码区块（shard）：服务端生成答案写入会话，刷新时仅本区块重渲染。
/// nonce 仅作刷新触发器；答案在服务端生成，参数不可信也不参与运算。
#[shard]
pub async fn CaptchaArea(cx: &Cx, locale: String, nonce: f64) -> Result<impl View> {
    let _ = nonce;
    let captcha = captcha::generate();
    let answer = captcha.answer.to_string();
    let _ = captcha::save_answer(cx, captcha.answer).await;
    let captcha_src = format!("data:image/svg+xml;base64,{}", STANDARD.encode(&captcha.svg));
    Ok(view! {
        <div class="flex-1 min-w-0 flex items-center gap-2">
            <div
                class="rounded overflow-hidden shrink-0"
                style="width:160px;height:40px;border:1px solid #d1d5db"
            >
                <img src=(captcha_src) alt="" class="w-full h-full object-cover">
            </div>
            <div class="relative flex-1 min-w-0">
                input(
                    attrs: attributes! {
                        type="text"
                        name="captcha_answer"
                        required=""
                        placeholder=(loader::t(&locale, "captcha_placeholder"))
                        oninput=(format!(
                            "var v=this.value.trim();var ok=v==='{answer}';var b=this.form.querySelector('button[type=submit]');b.disabled=!ok;var s=document.getElementById('cap-ok');if(s){{s.style.visibility=v?'visible':'hidden';s.textContent=ok?'\\u2713':'\\u2717';s.className=ok?'absolute right-2 top-1/2 -translate-y-1/2 text-green-500 font-bold':'absolute right-2 top-1/2 -translate-y-1/2 text-red-500 font-bold'}}",
                        ))
                        class="w-full h-10 text-center text-xl pr-8 placeholder:text-sm"
                    }
                )
                <span
                    id="cap-ok"
                    class="absolute right-2 top-1/2 -translate-y-1/2 text-green-500 font-bold"
                    style="visibility:hidden"
                ></span>
            </div>
        </div>
    })
}
