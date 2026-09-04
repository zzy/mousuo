#![allow(non_snake_case)]

use crate::common::constant::PRODUCT_LOW_STOCK_THRESHOLD;
use crate::common::format::format_cents;
use crate::components::badge::{BadgeVariant, badge};
use crate::components::status_badge::warning_badge;
use crate::i18n::loader;
use crate::models::product::Product;
use topcoat::{
    Result,
    view::{View, attributes, component, view},
};

/// 低库存文案（替换 {stock} 占位符）
fn low_stock_text(locale: &str, stock: i64) -> String {
    loader::t(locale, "product_low_stock").replace("{stock}", &stock.to_string())
}

/// 库存角标：库存充足正常色 / 售罄红色 / 低库存黄色
#[component]
pub async fn StockBadge(locale: String, stock: i64) -> Result<impl View> {
    Ok(view! {
        if stock == 0 {
            badge(
                variant: BadgeVariant::Destructive,
                attrs: attributes! {},
                (loader::t(&locale, "product_out_of_stock"))
            )
        } else if stock <= PRODUCT_LOW_STOCK_THRESHOLD {
            warning_badge(attrs: attributes! {}, (low_stock_text(&locale, stock)))
        } else {
            badge(
                variant: BadgeVariant::Secondary,
                attrs: attributes! {},
                (loader::t(&locale, "product_in_stock"))
            )
        }
    })
}

/// 商品卡片：4:3 图 + 名称 + 价格 + 库存角标
///
/// 铁律：商品图容器永远浅色底（bg-white 包裹图片，深色模式下不刺眼）
#[component]
pub async fn ProductCard(locale: String, product: Product) -> Result<impl View> {
    let price = format_cents(product.price_cents);
    Ok(view! {
        <a
            href=(format!("/{locale}/products/{}", product.slug))
            class="bg-surface border border-border rounded-lg shadow-xs overflow-hidden no-underline hover:shadow-md transition-shadow block"
        >
            <div class="aspect-4/3 bg-white relative">
                if let Some(ref img) = product.image {
                    <img
                        src=(img.clone())
                        alt=(product.title.clone())
                        class="w-full h-full object-cover"
                    >
                }
                <div class="absolute top-2 right-2">
                    StockBadge(locale: locale.clone(), stock: product.stock)
                </div>
            </div>
            <div class="p-3">
                <h3 class="font-medium text-sm text-foreground truncate">
                    (product.title.clone())
                </h3>
                <div class="text-base font-semibold text-foreground mt-1">(price)</div>
            </div>
        </a>
    })
}
