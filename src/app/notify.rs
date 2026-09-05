//! SSE 实时通知 — 登录用户订阅，内容更新时推送

use std::time::Duration;

use futures_util::stream;
use tokio::sync::broadcast;

use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        content::sse::{Event, KeepAlive, Sse},
        route,
    },
};

/// 广播通知（在 upload_handler 成功后调用）
pub fn notify(cx: &Cx, msg: &str) {
    let tx: &broadcast::Sender<String> = app_context(cx);
    let _ = tx.send(msg.to_string());
}

/// SSE 订阅端点
#[route(GET "/{locale}/notify")]
async fn subscribe(
    cx: &Cx,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event>> + use<>>> {
    let tx: &broadcast::Sender<String> = app_context(cx);
    let rx = tx.subscribe();
    let events = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(data) => Some((Ok(Event::new().data(data)), rx)),
            Err(_) => None,
        }
    });
    Ok(Sse::new(events).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}
