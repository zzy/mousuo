#![allow(non_snake_case)]

use crate::common::auth;
use crate::common::config;
use crate::common::constant::{
    ORDER_STATUS_CANCELLED, ORDER_STATUS_PENDING,
};
use crate::common::format::format_cents;
use crate::components;
use crate::components::button::{ButtonVariant, button_variants};
use crate::components::card::card;
use crate::components::order_badge::OrderStatusBadge;
use crate::db::{orders, products};
use crate::i18n::loader;
use crate::models::order::Order;
use crate::models::page::PageInfo;
use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param_segment, query_params},
    view::{attributes, class, component, view},
};

#[query_params]
pub struct OrdersQuery {
    pub page: Option<u64>,
    pub result: Option<String>,
}

/// 我的订单列表（LoginGuard 层保证已登录）
#[page("/{locale}/orders")]
pub async fn orders_list(cx: &Cx) -> Result {
    let locale = path_param_segment(cx, "locale");
    let loc = locale.to_string();
    let username = auth::current_user(cx)
        .await
        .unwrap_or_default();
    let params = query_params::<OrdersQuery>(cx).ok();
    let page = params.as_ref().and_then(|p| p.page).unwrap_or(1).max(1);
    let page_size = config::config().page_size as u64;
    let total = orders::count_orders(&username).await.unwrap_or(0);
    let page_info = PageInfo::new(total, page, page_size);
    let order_list = orders::list_orders(&username, page_info.current_page, page_size)
        .await
        .unwrap_or_default();
    let n = order_list.len();
    let locales: Vec<String> = std::iter::repeat(loc.clone()).take(n).collect();
    view! {
        <div class="max-w-4xl mx-auto px-4 py-8">
            <h1 class="text-xl font-bold mb-6 text-foreground">
                (loader::t(&locale, "my_orders"))
            </h1>
            if order_list.is_empty() {
                <div class="text-center py-16">
                    <p class="text-base text-muted-foreground">
                        (loader::t(&locale, "order_empty"))
                    </p>
                    <a
                        href=(format!("/{locale}/products"))
                        class="text-blue-600 dark:text-blue-400 hover:underline"
                    >
                        (loader::t(&locale, "nav_products"))
                    </a>
                </div>
            } else {
                <div class="space-y-3">
                    for (order, lc) in order_list.into_iter().zip(locales) {
                        OrderRow(locale: lc, order: order)
                    }
                </div>
                components::pagination::Pagination(
                    locale: loc,
                    page_info: page_info,
                    base_url: format!("/{locale}/orders")
                )
            }
        </div>
    }
}

#[component]
async fn OrderRow(locale: String, order: Order) -> Result {
    let created_date: String = order.created_at.chars().take(10).collect();
    view! {
        <a
            href=(format!("/{locale}/orders/{}", order.id))
            class="bg-surface border border-border rounded-lg shadow-xs overflow-hidden no-underline hover:shadow-md transition-shadow block"
        >
            <div class="p-4 flex items-center justify-between gap-4">
                <div class="min-w-0">
                    <div class="text-sm font-medium text-foreground truncate">
                        (order.id)
                    </div>
                    <div class="text-xs text-muted-foreground mt-1">
                        (loader::t(&locale, "order_created_at"))
                        " "
                        (created_date)
                    </div>
                </div>
                <div class="flex items-center gap-3 shrink-0">
                    OrderStatusBadge(
                        locale: locale.clone(),
                        status: order.status.clone()
                    )
                    <span class="text-base font-semibold text-foreground">
                        (format_cents(order.total_cents))
                    </span>
                </div>
            </div>
        </a>
    }
}

/// 支付结果页：GET /{locale}/orders/{id}?result=success|cancel
/// （LoginGuard 层保证已登录；本页校验订单归属，非本人 404）
#[page("/{locale}/orders/{id}")]
pub async fn order_detail(cx: &Cx) -> Result {
    let locale = path_param_segment(cx, "locale");
    let order_id = path_param_segment(cx, "id");
    let username = auth::current_user(cx)
        .await
        .unwrap_or_default();
    let found = orders::get_order_by_id(&order_id).await.ok().flatten();
    let is_owner = found
        .as_ref()
        .is_some_and(|o| o.user_id == username);
    let result = query_params::<OrdersQuery>(cx)
        .ok()
        .and_then(|p| p.result.clone())
        .unwrap_or_default();
    // 支付结果横幅文案键
    let banner_key = match result.as_str() {
        "success" => Some("payment_success"),
        "cancel" => Some("payment_cancel"),
        _ => None,
    };
    // 重试回链：查首个条目的商品 slug（商品已下架或不存在则回列表页）
    let retry_slug = match &found {
        Some(order) => match order.items.first() {
            Some(item) => products::get_product_by_id(&item.product_id)
                .await
                .ok()
                .flatten()
                .and_then(|p| (p.status == "active").then_some(p.slug)),
            None => None,
        },
        None => None,
    };
    view! {
        <div class="max-w-3xl mx-auto px-4 py-8">
            if !is_owner || found.is_none() {
                (topcoat::router::StatusCode::NOT_FOUND)
                <div class="text-center py-16">
                    <h1
                        class="text-7xl font-bold text-blue-600 dark:text-blue-400 mb-4"
                    >
                        "404"
                    </h1>
                    <p class="text-muted-foreground mb-4">
                        (loader::t(&locale, "page_error_404"))
                    </p>
                    <a
                        href=(format!("/{locale}/orders"))
                        class="text-blue-600 dark:text-blue-400 hover:underline"
                    >
                        (loader::t(&locale, "nav_orders"))
                    </a>
                </div>
            } else if let Some(ref order) = found {
                let created_date: String = order.created_at.chars().take(10).collect();
                <div class="space-y-4">
                    if let Some(key) = banner_key {
                        <div
                            class=(class!(
                                "p-4 rounded-lg border",
                                if result == "success" {
                                    "border-green-500 bg-green-500/10 text-green-700 dark:text-green-400"
                                } else {
                                    "border-border bg-foreground/5 text-foreground"
                                },
                            ))
                        >
                            (loader::t(&locale, key))
                        </div>
                    } else if order.status == ORDER_STATUS_PENDING {
                        <div
                            class="p-4 rounded-lg border border-border bg-foreground/5 text-foreground"
                        >
                            (loader::t(&locale, "payment_processing"))
                        </div>
                    }
                    card(
                        attrs: attributes! { class="p-6" },
                        <div class="flex items-center justify-between">
                            <h1 class="text-lg font-bold text-foreground">
                                (loader::t(&locale, "my_orders"))
                            </h1>
                            OrderStatusBadge(
                                locale: locale.to_string(),
                                status: order.status.clone()
                            )
                        </div>
                        <div class="text-xs text-muted-foreground">
                            (order.id.clone())
                        </div>
                        <div class="space-y-2 border-t border-border pt-4">
                            for item in &order.items {
                                <div class="flex items-center justify-between text-sm">
                                    <span class="text-foreground">
                                        (item.title.clone())
                                        " × "
                                        (item.qty)
                                    </span>
                                    <span class="text-foreground">
                                        (format_cents(item.price_cents * item.qty))
                                    </span>
                                </div>
                            }
                        </div>
                        <div
                            class="flex items-center justify-between border-t border-border pt-4"
                        >
                            <span class="text-sm text-muted-foreground">
                                (loader::t(&locale, "order_total"))
                            </span>
                            <span class="text-lg font-semibold text-foreground">
                                (format_cents(order.total_cents))
                            </span>
                        </div>
                        <div class="text-xs text-muted-foreground">
                            (loader::t(&locale, "order_created_at"))
                            " "
                            (created_date)
                        </div>
                        if order.status == ORDER_STATUS_CANCELLED
                            || result == "cancel" {
                            <a
                                href=(retry_slug
                                    .as_ref()
                                    .map(|slug| format!("/{locale}/products/{slug}"))
                                    .unwrap_or_else(|| format!("/{locale}/products")))
                                class=(button_variants(
                                    ButtonVariant::Primary,
                                    crate::components::button::ButtonSize::Md,
                                ))
                            >
                                (loader::t(&locale, "payment_retry"))
                            </a>
                        }
                    )
                </div>
            }
        </div>
    }
}
