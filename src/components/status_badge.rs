#![allow(non_snake_case)]

use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// 警示色徽章（项目自有组件）
///
/// badge.rs 由 `topcoat ui` 托管（`ui add --overwrite` 会以 registry 原版覆盖，
/// 本地扩展的变体会丢失），故业务定制的警示样式独立为本组件，勿写回 badge.rs。
/// 尺寸/圆角/字重与 badge 的 BASE 保持一致，配色为警示黄。
const WARNING: StaticClass = class!(
    "inline-flex w-fit shrink-0 items-center justify-center gap-1 rounded-md \
     border border-transparent bg-yellow-500 text-yellow-950 px-2 py-0.5 text-xs \
     font-medium whitespace-nowrap"
);

/// 警示徽章：待激活/低库存/未支付等需留意的状态
#[component]
pub async fn warning_badge(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span class=(class!(WARNING, attrs.remove("class"))) (attrs)>(child)</span>
    })
}
