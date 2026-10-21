use pulldown_cmark::{Options, Parser, html};

/// 将 Markdown 渲染为 HTML（纯函数，对齐 ftbsite server/markdown::render_md）
///
/// pulldown-cmark 原始 HTML 默认透传：正文可内嵌 <video>/<img> 媒体；
/// 信任边界：正文仅作者/管理员可写
pub fn render_md(md: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(md, opts));
    out
}

/// meta description 文本：Markdown 剥纯文本并截断至 155 字符
pub fn meta_description(md: &str) -> String {
    use pulldown_cmark::Event;
    let mut out = String::with_capacity(md.len());
    for event in Parser::new(md) {
        match event {
            Event::Text(t) | Event::Code(t) => {
                out.push_str(&t);
                out.push(' ');
            }
            Event::SoftBreak | Event::HardBreak => out.push(' '),
            _ => {}
        }
    }
    out.chars().take(155).collect()
}

/// JSON-LD 脚本内容：序列化后转义 < > &，防 UGC 注入与脚本闭合
pub fn json_ld(value: &impl serde::Serialize) -> String {
    let json = serde_json::to_string(value).unwrap_or_default();
    json.replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}
