#![allow(non_snake_case)]

use chrono::Datelike;

use crate::i18n::loader;
use topcoat::{
    Result,
    view::{View, component, view},
};

#[component]
pub async fn Footer(locale: String) -> Result<impl View> {
    let year = chrono::Utc::now().year();
    Ok(view! {
        <footer class="border-t border-border py-4 mt-auto">
            <div class="max-w-7xl mx-auto px-4 text-center">
                <p class="text-xs text-muted-foreground">
                    <a
                        href="https://github.com/zzy/mousuo"
                        target="_blank"
                        class="hover:underline"
                    >
                        (loader::t(&locale, "site_name"))
                    </a>
                    (format!(" © {year}"))
                </p>
            </div>
        </footer>
    })
}
