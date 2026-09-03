#![allow(non_snake_case)]

use crate::common::payment;
use topcoat::{
    Result,
    context::Cx,
    router::{Body, StatusCode, request::Bytes, response::Response},
};

/// Stripe 支付回调（DESIGN 四）：
/// 验签失败 400；成功处理返回 200；其余错误 400 + 日志
#[topcoat::router::route(POST "/webhook/stripe")]
pub async fn stripe_webhook(cx: &Cx, body: Bytes) -> Result<Response> {
    let signature = topcoat::router::request::headers(cx)
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let payload = String::from_utf8_lossy(&body);
    match payment::handle_webhook(&payload, signature).await {
        Ok(()) => Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .expect("构建 webhook 响应失败")),
        Err(e) => {
            eprintln!("stripe webhook 处理失败: {e}");
            Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::empty())
                .expect("构建 webhook 响应失败"))
        }
    }
}
