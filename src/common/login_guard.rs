use crate::common::auth;
use topcoat::{
    context::Cx,
    router::{
        Body, Layer, LayerFuture, Next, Path, StatusCode, header, path_param_segment,
        request::parts, response::Response,
    },
};

/// 登录守卫层：/{locale}/orders 下所有路由要求已登录，
/// 未登录 302 到 sign-in?next=当前页（登录成功后回跳）
pub struct LoginGuard;

impl Layer for LoginGuard {
    fn path(&self) -> Option<&Path> {
        Some(Path::new("/{locale}/orders"))
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            if auth::current_user(cx).await.is_some() {
                return next.run(cx, body).await;
            }
            let locale = path_param_segment(cx, "locale").to_string();
            let next_url = parts(cx)
                .uri
                .path_and_query()
                .map(|pq| pq.as_str().to_string())
                .unwrap_or_default();
            let location = format!("/{locale}/sign-in?next={next_url}");
            Ok(Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header(header::LOCATION, location)
                .body(Body::empty())
                .expect("构建登录守卫重定向响应失败"))
        })
    }
}
