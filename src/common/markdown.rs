use pulldown_cmark::{Options, Parser, html};

/// 将 Markdown 渲染为 HTML（纯函数，对齐 ftbsite server/markdown::render_md）
///
/// pulldown-cmark 原始 HTML 默认透传：商品描述可内嵌 <video>/<img> 媒体
/// （DESIGN 决策 14）；信任边界：商品描述仅管理员可写（P3 管理端），种子数据可信
pub fn render_md(md: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(md, opts));
    out
}
