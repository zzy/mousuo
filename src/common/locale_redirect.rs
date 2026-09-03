use crate::i18n::loader;
use topcoat::{
    context::Cx,
    router::{
        Body, Layer, LayerFuture, Method, Next, Path, StatusCode, header, request::parts,
        response::Response,
    },
};

/// 语言路径归一化层：非规范或不受支持的语言路径段 302 到规范路径
/// 仅处理 GET/HEAD；POST（表单、webhook）与 shard 请求不受影响
pub struct LocaleRedirect;

impl Layer for LocaleRedirect {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let request_parts = parts(cx);
            if request_parts.method == Method::GET || request_parts.method == Method::HEAD {
                let path = request_parts.uri.path();
                if let Some(new_path) = loader::normalize_locale_path(path) {
                    let location = match request_parts.uri.query() {
                        Some(q) if !q.is_empty() => format!("{new_path}?{q}"),
                        _ => new_path,
                    };
                    return Ok(Response::builder()
                        .status(StatusCode::SEE_OTHER)
                        .header(header::LOCATION, location)
                        .body(Body::empty())
                        .expect("构建语言路径重定向响应失败"));
                }
            }
            next.run(cx, body).await
        })
    }
}
