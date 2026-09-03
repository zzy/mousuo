#![allow(non_snake_case)]

use crate::i18n::loader;
use topcoat::{
    Result,
    view::{component, view},
};

/// 管理端子导航（订单/用户/商品/上传）
#[component]
pub async fn AdminNav(locale: String, active: String) -> Result {
    let items = [
        ("orders", "admin_orders"),
        ("users", "admin_users"),
        ("products", "admin_products"),
        ("upload", "admin_upload"),
    ];
    view! {
        <div class="flex flex-wrap gap-2 mb-6 border-b border-border pb-2">
            for (key, label) in items {
                <a
                    href=(format!("/{locale}/admin/{key}"))
                    class=(if key == active.as_str() {
                        "px-3 py-1.5 text-sm rounded-md font-medium no-underline bg-primary text-primary-foreground"
                    } else {
                        "px-3 py-1.5 text-sm rounded-md no-underline text-foreground hover:bg-muted"
                    })
                >
                    (loader::t(&locale, label))
                </a>
            }
        </div>
    }
}
