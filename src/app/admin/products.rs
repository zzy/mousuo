#![allow(non_snake_case)]

use crate::common::config;
use crate::common::constant::{PRODUCT_STATUS_ACTIVE, PRODUCT_STATUS_OFF};
use crate::common::form;
use crate::common::format::format_cents;
use crate::common::session;
use crate::components;
use crate::components::badge::{BadgeVariant, badge};
use crate::components::button::{ButtonSize, ButtonVariant, button_variants};
use crate::components::card::card;
use crate::components::csrf::CsrfField;
use crate::components::input::input;
use crate::components::label::label;
use crate::components::status_badge::warning_badge;
use crate::db::products;
use crate::i18n::loader;
use crate::models::page::PageInfo;
use crate::models::product::Product;
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{content::Form, error::forbidden, page, path_param_segment, query_params, response::Response},
    view::{View, attributes, component, view},
};

/// 商品表单（创建/编辑共用）
#[derive(Deserialize)]
pub struct ProductForm {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub price_cents: i64,
    pub stock: i64,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub csrf_token: String,
}

/// 仅含 CSRF 的表单（上下架/删除等无其他字段的 POST）
#[derive(Deserialize)]
pub struct CsrfForm {
    #[serde(default)]
    pub csrf_token: String,
}

/// 表单值（供组件回显）
pub struct ProductFormValues {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub price_cents: i64,
    pub stock: i64,
    pub image: String,
}

/// 表单校验；失败返回 error 键（供 ?error= 回显），editing_id 为编辑时自身排除
async fn validate_product(form: &ProductForm, editing_id: Option<&str>) -> Result<(), &'static str> {
    if !products::valid_slug(&form.slug) {
        return Err("slug_invalid");
    }
    if form.title.trim().is_empty() {
        return Err("title_invalid");
    }
    if form.description.trim().is_empty() {
        return Err("description_invalid");
    }
    if form.price_cents <= 0 {
        return Err("price_invalid");
    }
    if form.stock < 0 {
        return Err("stock_invalid");
    }
    if let Ok(Some(existing)) = products::get_product_by_slug_any(&form.slug).await {
        let is_self = editing_id.is_some_and(|id| id == existing.id);
        if !is_self {
            return Err("slug_exists");
        }
    }
    Ok(())
}

const FORM_ERROR_KEYS: [&str; 8] = [
    "slug_invalid",
    "slug_exists",
    "title_invalid",
    "description_invalid",
    "price_invalid",
    "stock_invalid",
    "create_failed",
    "update_failed",
];

#[query_params]
pub struct AdminProductsQuery {
    pub page: Option<u64>,
}

/// 商品管理列表（含下架）
#[page("/{locale}/admin/products")]
pub async fn admin_products_list(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let loc = locale.to_string();
    let params = query_params::<AdminProductsQuery>(cx).ok();
    let page = params.as_ref().and_then(|p| p.page).unwrap_or(1).max(1);
    let page_size = config::config().page_size as u64;
    let total = products::count_all_products().await.unwrap_or(0);
    let page_info = PageInfo::new(total, page, page_size);
    let product_list = products::list_all_products(page_info.current_page, page_size)
        .await
        .unwrap_or_default();
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    let n = product_list.len();
    let locales: Vec<String> = std::iter::repeat(loc.clone()).take(n).collect();
    let csrfs: Vec<String> = std::iter::repeat(csrf.clone()).take(n).collect();
    let notice = super::notice(
        cx,
        &locale,
        &[
            ("created", "admin_product_created"),
            ("updated", "admin_product_updated"),
            ("status", "admin_product_status_updated"),
            ("deleted", "admin_product_deleted"),
            ("invalid", "admin_product_invalid"),
        ],
    );
    Ok(view! {
        <div class="max-w-6xl mx-auto px-4 py-8">
            components::admin_nav::AdminNav(
                locale: loc.clone(),
                active: "products".to_string()
            )
            <div class="flex items-center justify-between mb-6">
                <h1 class="text-xl font-bold text-foreground">
                    (loader::t(&locale, "admin_products"))
                </h1>
                <a
                    href=(format!("/{locale}/admin/products/new"))
                    class=(button_variants(ButtonVariant::Primary, ButtonSize::Md))
                >
                    (loader::t(&locale, "admin_product_create"))
                </a>
            </div>
            if let Some(ref msg) = notice {
                <p class="text-sm text-green-600 mb-4">(msg.clone())</p>
            }
            if product_list.is_empty() {
                <p class="text-muted-foreground py-16 text-center">
                    (loader::t(&locale, "no_data"))
                </p>
            } else {
                <div class="space-y-2">
                    for ((product, lc), tok) in product_list
                        .into_iter()
                        .zip(locales)
                        .zip(csrfs) {
                        AdminProductRow(locale: lc, product: product, csrf: tok)
                    }
                </div>
                components::pagination::Pagination(
                    locale: loc,
                    page_info: page_info,
                    base_url: format!("/{locale}/admin/products")
                )
            }
        </div>
    })
}

/// 单行商品：标题/slug/价格/库存/状态 + 编辑/上下架/删除
#[component]
async fn AdminProductRow(locale: String, product: Product, csrf: String) -> Result<impl View> {
    let is_active = product.status == PRODUCT_STATUS_ACTIVE;
    let status_url = format!("/{locale}/admin/products/{}/status", product.id);
    let delete_url = format!("/{locale}/admin/products/{}/delete", product.id);
    let edit_url = format!("/{locale}/admin/products/{}/edit", product.id);
    Ok(view! {
        <div
            class="bg-surface border border-border rounded-lg p-4 flex flex-wrap items-center gap-3"
        >
            <div class="min-w-0 flex-1">
                <div class="text-sm font-medium text-foreground truncate">
                    (product.title.clone())
                </div>
                <div class="text-xs text-muted-foreground mt-0.5">
                    (product.slug.clone())
                    " · "
                    (format_cents(product.price_cents))
                    " · "
                    (loader::t(&locale, "admin_product_stock"))
                    " "
                    (product.stock)
                </div>
            </div>
            <div class="flex items-center gap-2">
                if is_active {
                    badge(
                        variant: BadgeVariant::Secondary,
                        attrs: attributes! {},
                        (loader::t(&locale, "product_status_active"))
                    )
                } else {
                    warning_badge(
                        attrs: attributes! {},
                        (loader::t(&locale, "product_status_off"))
                    )
                }
                <a
                    href=(edit_url)
                    class=(button_variants(ButtonVariant::Secondary, ButtonSize::Sm))
                >
                    (loader::t(&locale, "admin_product_edit"))
                </a>
                <form method="POST" action=(status_url) class="inline">
                    CsrfField(token: csrf.clone())
                    <button
                        type="submit"
                        class=(button_variants(ButtonVariant::Secondary, ButtonSize::Sm))
                    >
                        (loader::t(
                            &locale,
                            if is_active {
                                "admin_product_off"
                            } else {
                                "admin_product_on"
                            },
                        ))
                    </button>
                </form>
                <form method="POST" action=(delete_url) class="inline">
                    CsrfField(token: csrf)
                    <button
                        type="submit"
                        class=(button_variants(
                            ButtonVariant::Destructive,
                            ButtonSize::Sm,
                        ))
                        onclick=(format!(
                            "return confirm('{}')",
                            loader::t(&locale, "admin_confirm_delete"),
                        ))
                    >
                        (loader::t(&locale, "admin_product_delete"))
                    </button>
                </form>
            </div>
        </div>
    })
}

/// 创建页
#[page("/{locale}/admin/products/new")]
pub async fn admin_product_new_page(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let error_message = form::error_message(cx, &locale, &FORM_ERROR_KEYS);
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    Ok(view! {
        ProductFormView(
            locale: locale.to_string(),
            action_url: format!("/{locale}/admin/products"),
            heading: loader::t(&locale, "admin_product_create").to_string(),
            submit_label: loader::t(&locale, "admin_product_create").to_string(),
            values: ProductFormValues {
                slug: String::new(),
                title: String::new(),
                description: String::new(),
                price_cents: 0,
                stock: 0,
                image: String::new(),
            },
            error_message: error_message,
            csrf: csrf
        )
    })
}

/// 编辑页
#[page("/{locale}/admin/products/{id}/edit")]
pub async fn admin_product_edit_page(cx: &Cx) -> Result<impl View> {
    let locale = path_param_segment(cx, "locale");
    let product_id = path_param_segment(cx, "id");
    let error_message = form::error_message(cx, &locale, &FORM_ERROR_KEYS);
    let csrf = session::ensure_csrf_token(cx).await.unwrap_or_default();
    let found = products::get_product_by_id(&product_id).await.ok().flatten();
    Ok(view! {
        if let Some(product) = found {
            ProductFormView(
                locale: locale.to_string(),
                action_url: format!("/{locale}/admin/products/{product_id}"),
                heading: loader::t(&locale, "admin_product_edit").to_string(),
                submit_label: loader::t(&locale, "admin_form_save").to_string(),
                values: ProductFormValues {
                    slug: product.slug,
                    title: product.title,
                    description: product.description,
                    price_cents: product.price_cents,
                    stock: product.stock,
                    image: product.image.unwrap_or_default(),
                },
                error_message: error_message,
                csrf: csrf
            )
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

/// 商品表单组件（创建/编辑共用）
#[component]
async fn ProductFormView(
    locale: String,
    action_url: String,
    heading: String,
    submit_label: String,
    values: ProductFormValues,
    error_message: Option<String>,
    csrf: String,
) -> Result<impl View> {
    Ok(view! {
        <div class="max-w-3xl mx-auto px-4 py-8">
            components::admin_nav::AdminNav(
                locale: locale.clone(),
                active: "products".to_string()
            )
            <h1 class="text-xl font-bold mb-6 text-foreground">(heading)</h1>
            if let Some(ref msg) = error_message {
                <p class="text-red-500 text-sm mb-4">(msg.clone())</p>
            }
            card(
                attrs: attributes! { class="p-6" },
                <form method="POST" action=(action_url) class="space-y-4">
                    CsrfField(token: csrf.clone())
                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                        <div class="space-y-1">
                            label(
                                attrs: attributes! {},
                                (loader::t(&locale, "admin_form_title"))
                            )
                            input(
                                attrs: attributes! {
                                    type="text"
                                    name="title"
                                    required=""
                                    value=(values.title.clone())
                                }
                            )
                        </div>
                        <div class="space-y-1">
                            label(
                                attrs: attributes! {},
                                (loader::t(&locale, "admin_form_slug"))
                            )
                            input(
                                attrs: attributes! {
                                    type="text"
                                    name="slug"
                                    required=""
                                    value=(values.slug.clone())
                                }
                            )
                        </div>
                        <div class="space-y-1">
                            label(
                                attrs: attributes! {},
                                (loader::t(&locale, "admin_form_price_cents"))
                            )
                            input(
                                attrs: attributes! {
                                    type="number"
                                    name="price_cents"
                                    min="1"
                                    required=""
                                    value=(values.price_cents.to_string())
                                }
                            )
                        </div>
                        <div class="space-y-1">
                            label(
                                attrs: attributes! {},
                                (loader::t(&locale, "admin_form_stock"))
                            )
                            input(
                                attrs: attributes! {
                                    type="number"
                                    name="stock"
                                    min="0"
                                    required=""
                                    value=(values.stock.to_string())
                                }
                            )
                        </div>
                    </div>
                    <div class="space-y-1">
                        label(
                            attrs: attributes! {},
                            (loader::t(&locale, "admin_form_image_url"))
                        )
                        input(
                            attrs: attributes! {
                                type="text"
                                name="image"
                                placeholder="/media/…"
                                value=(values.image.clone())
                            }
                        )
                        <p class="text-xs text-muted-foreground">
                            (loader::t(&locale, "admin_form_image_hint"))
                            " "
                            <a
                                href=(format!("/{locale}/admin/upload"))
                                target="_blank"
                                class="text-blue-600 dark:text-blue-400 hover:underline"
                            >
                                (loader::t(&locale, "admin_upload"))
                            </a>
                        </p>
                    </div>
                    <div class="space-y-1">
                        label(
                            attrs: attributes! {},
                            (loader::t(&locale, "admin_form_description"))
                        )
                        components::markdown_editor::MarkdownEditor(
                            locale: locale.clone(),
                            name: "description".to_string(),
                            rows: 12,
                            value: values.description.clone(),
                            required: true,
                            csrf: csrf.clone()
                        )
                    </div>
                    <button
                        type="submit"
                        class=(button_variants(ButtonVariant::Primary, ButtonSize::Md))
                    >
                        (submit_label)
                    </button>
                </form>
            )
        </div>
    })
}

/// 创建提交
#[topcoat::router::route(POST "/{locale}/admin/products")]
pub async fn admin_product_create(cx: &Cx, Form(form): Form<ProductForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let error_url = format!("/{locale}/admin/products/new");
    if let Err(key) = validate_product(&form, None).await {
        return Ok(form::redirect(&(error_url + "?error=" + key)));
    }
    let image = form.image.clone();
    match products::create_product(
        &form.slug,
        &form.title,
        &form.description,
        form.price_cents,
        form.stock,
        Some(&image),
    )
    .await
    {
        Ok(_) => Ok(form::redirect(&format!("/{locale}/admin/products?ok=created"))),
        Err(_) => Ok(form::redirect(&(error_url + "?error=create_failed"))),
    }
}

/// 编辑提交
#[topcoat::router::route(POST "/{locale}/admin/products/{id}")]
pub async fn admin_product_update(cx: &Cx, Form(form): Form<ProductForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let product_id = path_param_segment(cx, "id");
    let error_url = format!("/{locale}/admin/products/{product_id}/edit");
    if let Err(key) = validate_product(&form, Some(&product_id)).await {
        return Ok(form::redirect(&(error_url + "?error=" + key)));
    }
    let image = form.image.clone();
    match products::update_product(
        &product_id,
        &form.slug,
        &form.title,
        &form.description,
        form.price_cents,
        form.stock,
        Some(&image),
    )
    .await
    {
        Ok(()) => Ok(form::redirect(&format!("/{locale}/admin/products?ok=updated"))),
        Err(_) => Ok(form::redirect(&(error_url + "?error=update_failed"))),
    }
}

/// 上架/下架切换
#[topcoat::router::route(POST "/{locale}/admin/products/{id}/status")]
pub async fn admin_product_status(cx: &Cx, Form(form): Form<CsrfForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let product_id = path_param_segment(cx, "id");
    let base_url = format!("/{locale}/admin/products");
    if let Some(product) = products::get_product_by_id(&product_id).await.ok().flatten() {
        let next = if product.status == PRODUCT_STATUS_ACTIVE {
            PRODUCT_STATUS_OFF
        } else {
            PRODUCT_STATUS_ACTIVE
        };
        if products::set_product_status(&product_id, next).await.is_ok() {
            return Ok(form::redirect(&(base_url + "?ok=status")));
        }
    }
    Ok(form::redirect(&(base_url + "?ok=invalid")))
}

/// 删除提交
#[topcoat::router::route(POST "/{locale}/admin/products/{id}/delete")]
pub async fn admin_product_delete(cx: &Cx, Form(form): Form<CsrfForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let product_id = path_param_segment(cx, "id");
    let base_url = format!("/{locale}/admin/products");
    match products::delete_product(&product_id).await {
        Ok(()) => Ok(form::redirect(&(base_url + "?ok=deleted"))),
        Err(_) => Ok(form::redirect(&(base_url + "?ok=invalid"))),
    }
}
