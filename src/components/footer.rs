#![allow(non_snake_case)]

use chrono::Datelike;

use crate::i18n::loader;
use topcoat::{
    Result,
    view::{component, view},
};

#[component]
pub async fn Footer(locale: String) -> Result {
    let year = chrono::Utc::now().year();
    view! {
        <footer class="border-t border-border py-4 mt-auto">
            <div class="max-w-7xl mx-auto px-4 text-center">
                <p class="text-xs text-muted-foreground">
                    (loader::t(&locale, "site_name"))
                    (format!(" © {year}"))
                </p>
            </div>
        </footer>
    }
}
