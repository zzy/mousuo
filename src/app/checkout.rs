#![allow(non_snake_case)]

use crate::common::{auth, config, form, payment, session};
use crate::db::{orders, products};
use crate::models::order::OrderItem;
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{content::Form, error::forbidden, path_param_segment, response::Response},
};

#[derive(Deserialize)]
pub struct CheckoutForm {
    pub product_id: String,
    /// 回跳路径（登录守卫用），来自商品详情页隐藏域
    #[serde(default)]
    pub next: String,
    #[serde(default)]
    pub csrf_token: String,
}

/// 下单 + 跳支付（DESIGN 四）：
/// 商品校验 → 登录守卫 → 建 pending 单 → 建 Stripe Session → 回写会话号 → 302 支付页
#[topcoat::router::route(POST "/{locale}/checkout")]
pub async fn checkout_action(cx: &Cx, Form(form): Form<CheckoutForm>) -> Result<Response> {
    let locale = path_param_segment(cx, "locale");
    if !session::verify_csrf(cx, &form.csrf_token).await {
        return Err(forbidden().into());
    }
    let cfg = config::config();
    // 1. 商品校验：存在 + active + stock > 0
    let product = match products::get_product_by_id(&form.product_id).await {
        Ok(Some(p)) => p,
        _ => {
            return Ok(form::redirect(&format!(
                "/{locale}/products?error=invalid"
            )))
        }
    };
    if product.status != "active" {
        return Ok(form::redirect(&format!(
            "/{locale}/products/{}?error=invalid",
            product.slug
        )));
    }
    let detail_url = format!("/{locale}/products/{}", product.slug);
    if product.stock <= 0 {
        return Ok(form::redirect(&format!("{detail_url}?error=stock")));
    }
    // 2. 登录守卫：未登录 → sign-in?next=商品页
    let username = match auth::current_user(cx).await {
        Some(u) => u,
        None => {
            let next = form::safe_next(&form.next).unwrap_or_else(|| detail_url.clone());
            return Ok(form::redirect(&format!(
                "/{locale}/sign-in?next={next}"
            )));
        }
    };
    // 3. 建 pending 单（条目快照，试水单件）
    let items = vec![OrderItem {
        product_id: product.id.clone(),
        title: product.title.clone(),
        price_cents: product.price_cents,
        qty: 1,
    }];
    let order = match orders::create_order(&username, items, product.price_cents).await {
        Ok(o) => o,
        Err(e) => {
            eprintln!("checkout 建单失败: {e}");
            return Ok(form::redirect(&format!("{detail_url}?error=payment")));
        }
    };
    // 4. 建支付会话（success/cancel 用请求 scheme+domain 拼接，含本地端口）
    let scheme = topcoat::router::request::headers(cx)
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let success_url = format!(
        "{scheme}://{}/{locale}/orders/{}?result=success",
        cfg.domain, order.id
    );
    let cancel_url = format!(
        "{scheme}://{}/{locale}/orders/{}?result=cancel",
        cfg.domain, order.id
    );
    let (pay_url, session_id) =
        match payment::create_checkout(&order, &success_url, &cancel_url).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("checkout 建支付会话失败（订单保留 pending）: {e}");
                return Ok(form::redirect(&format!("{detail_url}?error=payment")));
            }
        };
    // 5. 回写会话号（关联键，webhook 不靠 metadata）
    if let Err(e) = orders::set_session_id(&order.id, &session_id).await {
        eprintln!("回写 stripe_session_id 失败: {e}");
    }
    // 6. 302 → Stripe 支付页
    Ok(form::redirect(&pay_url))
}
