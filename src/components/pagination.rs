#![allow(non_snake_case)]

use crate::components::button::{ButtonSize, ButtonVariant, button_variants};
use crate::i18n::loader;
use crate::models::page::PageInfo;
use topcoat::{
    Result,
    view::{View, component, view},
};

/// 分页导航组件
#[component]
pub async fn Pagination(locale: String, page_info: PageInfo, base_url: String) -> Result<impl View> {
    let sep = if base_url.contains('?') { "&" } else { "?" };
    let prev_url = format!(
        "{base_url}{sep}page={}",
        page_info.current_page.saturating_sub(1).max(1)
    );
    let next_url = format!("{base_url}{sep}page={}", page_info.current_page + 1);
    let info_text = format!(
        "{} {}/{}（{} {}）",
        loader::t(&locale, "pagination_page"),
        page_info.current_page,
        page_info.total_pages,
        page_info.total_count,
        loader::t(&locale, "pagination_items"),
    );
    Ok(view! {
        if page_info.total_pages > 1 {
            <nav class="flex items-center justify-between my-8 px-4">
                <div class="text-sm text-muted-foreground">(info_text)</div>
                <div class="flex gap-2">
                    if page_info.has_previous {
                        <a
                            href=(prev_url)
                            class=(button_variants(
                                ButtonVariant::Secondary,
                                ButtonSize::Sm,
                            ))
                        >
                            (loader::t(&locale, "pagination_previous"))
                        </a>
                    } else {
                        <span
                            class="py-1.5 px-3 bg-muted cursor-not-allowed text-sm rounded"
                        >
                            (loader::t(&locale, "pagination_previous"))
                        </span>
                    }
                    if page_info.has_next {
                        <a
                            href=(next_url)
                            class=(button_variants(
                                ButtonVariant::Secondary,
                                ButtonSize::Sm,
                            ))
                        >
                            (loader::t(&locale, "pagination_next"))
                        </a>
                    } else {
                        <span
                            class="py-1.5 px-3 bg-muted cursor-not-allowed text-sm rounded"
                        >
                            (loader::t(&locale, "pagination_next"))
                        </span>
                    }
                </div>
            </nav>
        }
    })
}
