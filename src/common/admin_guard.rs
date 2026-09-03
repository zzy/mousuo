use crate::common::auth;
use crate::i18n::loader;
use topcoat::{
    context::Cx,
    router::{
        Body, Layer, LayerFuture, Next, Path, StatusCode, header, path_param_segment,
        request::parts, response::Response,
    },
};

/// 管理员守卫层：/{locale}/admin 下所有路由要求管理员，
/// 未登录 302 到 sign-in?next=当前页；已登录非管理员 404（验收关卡）
pub struct AdminGuard;

impl Layer for AdminGuard {
    fn path(&self) -> Option<&Path> {
        Some(Path::new("/{locale}/admin"))
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            // 未登录：与 LoginGuard 同款回跳
            if auth::current_user(cx).await.is_none() {
                let locale = path_param_segment(cx, "locale").to_string();
                let next_url = parts(cx)
                    .uri
                    .path_and_query()
                    .map(|pq| pq.as_str().to_string())
                    .unwrap_or_default();
                let location = format!("/{locale}/sign-in?next={next_url}");
                return Ok(Response::builder()
                    .status(StatusCode::SEE_OTHER)
                    .header(header::LOCATION, location)
                    .body(Body::empty())
                    .expect("构建管理员守卫重定向响应失败"));
            }
            // 已登录非管理员：404（不泄露管理端存在性细节）
            if !auth::is_admin(cx).await {
                let locale = path_param_segment(cx, "locale").to_string();
                let body_text = loader::t(&locale, "page_error_404");
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                    .body(Body::from(body_text.to_string()))
                    .expect("构建管理员守卫 404 响应失败"));
            }
            next.run(cx, body).await
        })
    }
}
