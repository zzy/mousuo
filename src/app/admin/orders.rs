#![allow(non_snake_case)]

use crate::common::{config, form, order_state, session};
use crate::components;
use crate::components::button::{ButtonSize, ButtonVariant, button_variants};
use crate::components::card::card;
use crate::components::csrf::CsrfField;
use crate::components::order_badge::OrderStatusBadge;
use crate::db::orders;
use crate::i18n::loader;
use crate::models::order::Order;
use crate::models::page::PageInfo;
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{content::Form, error::forbidden, page, path_param_segment, query_params, response::Response},
    view::{View, attributes, component, view},
};

#[query_params]
pub struct AdminOrdersQuery {
    pub page: Option<u64>,
}

/// 管理订单列表（AdminGuard 层保证管理员）
#[page("/{locale}/admin/orders")]
pub async fn admin_orders_list(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let loc = locale.to_string();
    let params = query_params::<AdminOrdersQuery>(cx).ok();
    let page = params.as_ref().and_then(|p| p.page).unwrap_or(1).max(1);
    let page_size = config::config().page_size as u64;
    let total = orders::count_all_orders().await.unwrap_or(0);
    let page_info = PageInfo::new(total, page, page_size);
    let order_list = orders::list_all_orders(page_info.current_page, page_size)
        .await
        .unwrap_or_default();
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    let n = order_list.len();
    let locales: Vec<String> = std::iter::repeat(loc.clone()).take(n).collect();
    let csrfs: Vec<String> = std::iter::repeat(csrf.clone()).take(n).collect();
    let notice = super::notice(
        cx,
        &locale,
        &[
            ("updated", "admin_order_updated"),
            ("invalid", "admin_order_invalid"),
        ],
    );
    Ok(view! {
        <div class="max-w-6xl mx-auto px-4 py-8">
            components::admin_nav::AdminNav(
                locale: loc.clone(),
                active: "orders".to_string()
            )
            <h1 class="text-xl font-bold mb-6 text-foreground">
                (loader::t(&locale, "admin_orders"))
            </h1>
            if let Some(ref msg) = notice {
                <p class="text-sm text-green-600 mb-4">(msg.clone())</p>
            }
            if order_list.is_empty() {
                <p class="text-muted-foreground py-16 text-center">
                    (loader::t(&locale, "no_data"))
                </p>
            } else {
                <div class="space-y-2">
                    for ((order, lc), tok) in order_list
                        .into_iter()
                        .zip(locales)
                        .zip(csrfs) {
                        AdminOrderRow(locale: lc, order: order, csrf: tok)
                    }
                </div>
                components::pagination::Pagination(
                    locale: loc,
                    page_info: page_info,
                    base_url: format!("/{locale}/admin/orders")
                )
            }
        </div>
    })
}

/// 单行订单：id（链到详情）、用户、条目摘要、金额、状态、管理动作
#[component]
async fn AdminOrderRow(locale: String, order: Order, csrf: String) -> Result<impl View> {
    let created_date: String = order.created_at.chars().take(10).collect();
    let item_summary = order
        .items
        .first()
        .map(|item| format!("{} × {}", item.title, item.qty))
        .unwrap_or_else(|| loader::t(&locale, "no_data").to_string());
    let extra_count = order.items.len().saturating_sub(1);
    let detail_url = format!("/{locale}/admin/orders/{}", order.id);
    Ok(view! {
        <div
            class="bg-surface border border-border rounded-lg p-4 flex flex-wrap items-center gap-3"
        >
            <div class="min-w-0 flex-1">
                <div class="text-sm font-medium text-foreground truncate">
                    <a
                        href=(detail_url)
                        class="text-blue-600 dark:text-blue-400 hover:underline no-underline"
                    >
                        (order.id.clone())
                    </a>
                </div>
                <div class="text-xs text-muted-foreground mt-0.5">
                    (order.user_id.clone())
                    " · "
                    (item_summary)
                    if extra_count > 0 {
                        " +"
                        (extra_count)
                    }
                    " · "
                    (created_date)
                </div>
            </div>
            <div class="flex items-center gap-2">
                <span class="text-sm font-semibold text-foreground">
                    (crate::common::format::format_cents(order.total_cents))
                </span>
                OrderStatusBadge(locale: locale.clone(), status: order.status.clone())
                AdminOrderActions(
                    locale: locale.clone(),
                    order_id: order.id.clone(),
                    status: order.status.clone(),
                    csrf: csrf,
                    back: "list".to_string()
                )
            </div>
        </div>
    })
}

/// 管理动作按钮组（列表与详情共用）；back=list|detail 决定流转后回跳目标
#[component]
async fn AdminOrderActions(
    locale: String,
    order_id: String,
    status: String,
    csrf: String,
    back: String,
) -> Result<impl View> {
    let actions = order_state::next_actions(&status);
    let actions_len = actions.len();
    let action_locales: Vec<String> = std::iter::repeat(locale.clone()).take(actions_len).collect();
    let form_action = format!("/{locale}/admin/orders/{order_id}/status?back={back}");
    Ok(view! {
        if !actions.is_empty() {
            <form
                method="POST"
                action=(form_action)
                class="inline-flex items-center gap-2"
            >
                CsrfField(token: csrf)
                for (action, lc) in actions.iter().zip(action_locales) {
                    <button
                        type="submit"
                        name="to"
                        value=(action.to_string())
                        class=(button_variants(ButtonVariant::Secondary, ButtonSize::Sm))
                        onclick=(format!(
                            "return confirm('{}')",
                            loader::t(&lc, "admin_confirm_transition"),
                        ))
                    >
                        (loader::t(&lc, action))
                    </button>
                }
            </form>
        }
    })
}

/// 管理订单详情：完整条目、时间线、金额与流转动作
#[page("/{locale}/admin/orders/{id}")]
pub async fn admin_order_detail(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let order_id = path_param_segment(cx, "id");
    let found = orders::get_order_by_id(&order_id).await.ok().flatten();
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    let notice = super::notice(cx, &locale, &[("updated", "admin_order_updated")]);
    // 日期字段仅在命中时展示，提前按 Option 计算以合并为单一 view! 返回
    let created_date = found
        .as_ref()
        .map(|o| o.created_at.chars().take(10).collect::<String>())
        .unwrap_or_default();
    let paid_date = found
        .as_ref()
        .and_then(|o| o.paid_at.as_deref())
        .map(|d| d.chars().take(10).collect::<String>())
        .unwrap_or_else(|| "-".to_string());
    let cancelled_date = found
        .as_ref()
        .and_then(|o| o.cancelled_at.as_deref())
        .map(|d| d.chars().take(10).collect::<String>())
        .unwrap_or_else(|| "-".to_string());
    Ok(view! {
        if let Some(order) = found {
            <div class="max-w-4xl mx-auto px-4 py-8">
                components::admin_nav::AdminNav(
                    locale: locale.to_string(),
                    active: "orders".to_string()
                )
                <div class="flex items-center justify-between mb-6">
                    <h1 class="text-xl font-bold text-foreground">
                        (loader::t(&locale, "admin_order_detail"))
                    </h1>
                    <a
                        href=(format!("/{locale}/admin/orders"))
                        class="text-blue-600 dark:text-blue-400 hover:underline no-underline text-sm"
                    >
                        (loader::t(&locale, "admin_order_back"))
                    </a>
                </div>
                if let Some(ref msg) = notice {
                    <p class="text-sm text-green-600 mb-4">(msg.clone())</p>
                }
                card(
                    attrs: attributes! { class="p-6" },
                    <div class="flex flex-wrap items-center gap-3 justify-between">
                        <div class="min-w-0">
                            <div
                                class="text-sm font-mono text-muted-foreground break-all"
                            >
                                (order.id.clone())
                            </div>
                            <div class="text-sm text-foreground mt-1">
                                (loader::t(&locale, "admin_order_user"))
                                ": "
                                (order.user_id.clone())
                            </div>
                        </div>
                        <div class="flex items-center gap-2">
                            OrderStatusBadge(
                                locale: locale.to_string(),
                                status: order.status.clone()
                            )
                            AdminOrderActions(
                                locale: locale.to_string(),
                                order_id: order.id.clone(),
                                status: order.status.clone(),
                                csrf: csrf,
                                back: "detail".to_string()
                            )
                        </div>
                    </div>
                    <div
                        class="grid grid-cols-2 sm:grid-cols-4 gap-4 border-t border-border mt-4 pt-4 text-sm"
                    >
                        <div>
                            <div class="text-xs text-muted-foreground">
                                (loader::t(&locale, "order_created_at"))
                            </div>
                            <div class="text-foreground mt-0.5">(created_date)</div>
                        </div>
                        <div>
                            <div class="text-xs text-muted-foreground">
                                (loader::t(&locale, "admin_order_paid_at"))
                            </div>
                            <div class="text-foreground mt-0.5">(paid_date)</div>
                        </div>
                        <div>
                            <div class="text-xs text-muted-foreground">
                                (loader::t(&locale, "admin_order_cancelled_at"))
                            </div>
                            <div class="text-foreground mt-0.5">(cancelled_date)</div>
                        </div>
                        <div>
                            <div class="text-xs text-muted-foreground">
                                (loader::t(&locale, "order_total"))
                            </div>
                            <div class="text-foreground mt-0.5 font-semibold">
                                (crate::common::format::format_cents(order.total_cents))
                            </div>
                        </div>
                    </div>
                    if let Some(ref sid) = order.stripe_session_id {
                        <div
                            class="text-xs text-muted-foreground border-t border-border mt-4 pt-3 break-all"
                        >
                            (loader::t(&locale, "admin_order_stripe_session"))
                            ": "
                            (sid.clone())
                        </div>
                    }
                )
                <h2 class="text-base font-semibold text-foreground mt-8 mb-3">
                    (loader::t(&locale, "admin_order_items"))
                </h2>
                <div class="space-y-2">
                    for item in order.items.iter() {
                        <div
                            class="bg-surface border border-border rounded-lg px-4 py-3 flex items-center justify-between gap-3"
                        >
                            <div class="min-w-0">
                                <div class="text-sm text-foreground truncate">
                                    (item.title.clone())
                                </div>
                                <div class="text-xs text-muted-foreground mt-0.5">
                                    (crate::common::format::format_cents(item.price_cents))
                                    " × "
                                    (item.qty)
                                </div>
                            </div>
                            <div class="text-sm font-semibold text-foreground shrink-0">
                                (crate::common::format::format_cents(
                                    item.price_cents * item.qty,
                                ))
                            </div>
                        </div>
                    }
                </div>
            </div>
        } else {
            (topcoat::router::StatusCode::NOT_FOUND)
            <div class="max-w-3xl mx-auto px-4 py-16 text-center">
                <h1 class="text-2xl font-bold text-foreground mb-4">"404"</h1>
                <p class="text-muted-foreground">
                    (loader::t(&locale, "page_error_404"))
                </p>
            </div>
        }
    })
}

#[derive(Deserialize)]
pub struct StatusForm {
    pub to: String,
    #[serde(default)]
    pub csrf_token: String,
}

/// 状态流转（管理端）：状态机校验 → 条件原子更新 → 按 back 参数回跳（列表/详情）
#[topcoat::router::route(POST "/{locale}/admin/orders/{id}/status")]
pub async fn admin_order_status(cx: &Cx, Form(form): Form<StatusForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let order_id = path_param_segment(cx, "id");
    let back_detail = form::query_param(cx, "back").as_deref() == Some("detail");
    let ok_url = if back_detail {
        format!("/{locale}/admin/orders/{order_id}?ok=updated")
    } else {
        format!("/{locale}/admin/orders?ok=updated")
    };
    let invalid_url = if back_detail {
        format!("/{locale}/admin/orders/{order_id}?ok=invalid")
    } else {
        format!("/{locale}/admin/orders?ok=invalid")
    };
    let Some(order) = orders::get_order_by_id(&order_id).await.ok().flatten() else {
        return Ok(form::redirect(&invalid_url));
    };
    if !order_state::allowed_transition(&order.status, &form.to) {
        return Ok(form::redirect(&invalid_url));
    }
    match orders::transition_status(&order_id, &order.status, &form.to).await {
        Ok(true) => Ok(form::redirect(&ok_url)),
        _ => Ok(form::redirect(&invalid_url)),
    }
}
