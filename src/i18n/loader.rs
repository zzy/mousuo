include!(concat!(env!("OUT_DIR"), "/i18n_gen.rs"));

use icu::locale::{
    Locale, LocaleCanonicalizer, LocaleExpander, LocaleFallbacker,
    fallback::{LocaleFallbackConfig, LocaleFallbackPriority},
};
use topcoat::{context::Cx, cookie::Cookies};

/// 语言标识规范化器（CLDR 别名/大小写/旧式区域码，compiled_data 烘焙）
static CANONICALIZER: LazyLock<LocaleCanonicalizer<LocaleExpander>> =
    LazyLock::new(LocaleCanonicalizer::new_common);

/// 语言最大化器（CLDR likely subtags，用于判定「真实语言」与普通路径段的区别）
static EXPANDER: LazyLock<LocaleExpander> = LazyLock::new(LocaleExpander::new_common);

/// 解析并规范化语言标识；空串或非法输入返回 None
fn parse_locale(input: &str) -> Option<Locale> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut locale = Locale::try_from_str(trimmed).ok()?;
    CANONICALIZER.canonicalize(&mut locale);
    Some(locale)
}

/// 判断输入是否为「真实语言」而非普通路径段：
/// 能解析，且经 CLDR 最大化后带脚本或区域（真实语言必有 likely subtags；
/// assets/webhook/favicon 等非语言段最大化后仍无脚本区域，予以放行）
fn is_real_locale(input: &str) -> bool {
    let Some(mut locale) = parse_locale(input) else {
        return false;
    };
    EXPANDER.maximize(&mut locale.id);
    locale.id.script.is_some() || locale.id.region.is_some()
}

/// 沿 ICU 回退链（语言优先）寻找受支持语言，返回受支持语言码；
/// 全链落空返回 None
fn resolve_supported(locale: &Locale) -> Option<String> {
    let fallbacker = LocaleFallbacker::new();
    let mut config = LocaleFallbackConfig::default();
    config.priority = LocaleFallbackPriority::Language;
    let mut chain = fallbacker
        .for_config(config)
        .fallback_for(locale.into());
    loop {
        let current = chain.get();
        let full = current.to_string();
        if is_supported(&full) {
            return Some(full);
        }
        let lang = current.language.as_str();
        if is_supported(lang) {
            return Some(lang.to_string());
        }
        if lang == "und" {
            return None;
        }
        chain.step();
    }
}

/// 从请求 URL 路径提取语言标识（首段，经 ICU 解析与规范化），
/// 如 /zh/sign-in → "zh"；非法或不支持时兜底 detect（cookie > 协商 > 默认）
pub fn locale_from_path(cx: &Cx) -> String {
    let path = topcoat::router::request::parts(cx).uri.path();
    let first = path.split('/').nth(1).unwrap_or("");
    parse_locale(first)
        .as_ref()
        .and_then(resolve_supported)
        .unwrap_or_else(|| detect(cx))
}

/// Accept-Language 协商（RFC 9110 打分制，纯函数便于测试）：
/// 对每个受支持语言取「最长匹配 range」的 q 值——显式点名取最长 range 的 q，
/// 未点名语言吃通配 q，q=0 即排除。取最高分；同分依次按「显式点名优先于
/// 通配命中、默认语言优先」裁决；全部被排除时返回 None，调用方兜底默认语言。
pub fn negotiate_language(header: &str) -> Option<String> {
    // (range, q)：None 表示通配 *；非法 q 段剔除
    let mut ranges: Vec<(Option<Locale>, f64)> = Vec::new();
    for part in header.split(',') {
        let mut segments = part.split(';');
        let range = segments.next().unwrap_or("").trim();
        if range.is_empty() {
            continue;
        }
        let mut q = 1.0_f64;
        let mut valid = true;
        for param in segments {
            let param = param.trim();
            if let Some(value) = param.strip_prefix("q=") {
                match value.trim().parse::<f64>() {
                    Ok(parsed) if (0.0..=1.0).contains(&parsed) => q = parsed,
                    _ => {
                        valid = false;
                        break;
                    }
                }
            }
        }
        if !valid {
            continue;
        }
        if range == "*" {
            ranges.push((None, q));
        } else if let Some(locale) = parse_locale(range) {
            ranges.push((Some(locale), q));
        }
    }
    if ranges.is_empty() {
        return None;
    }
    let wildcard_q = ranges
        .iter()
        .filter(|(r, _)| r.is_none())
        .map(|(_, q)| *q)
        .fold(None, |acc: Option<f64>, q| {
            Some(acc.map_or(q, |a| a.max(q)))
        });
    // (语言码, q, 是否显式点名)；同分同级平局时默认语言胜
    let mut best: Option<(String, f64, bool)> = None;
    for lang in SUPPORTED {
        // 显式命名的 range：匹配语言部分（站点仅两级语言码，脚本/区域只在长度上加权）
        let explicit: Vec<(usize, f64)> = ranges
            .iter()
            .filter_map(|(r, q)| match r {
                Some(locale) if locale.id.language.as_str() == *lang => Some((
                    1 + usize::from(locale.id.script.is_some())
                        + usize::from(locale.id.region.is_some()),
                    *q,
                )),
                _ => None,
            })
            .collect();
        let (q, explicit_hit) = if explicit.is_empty() {
            wildcard_q.map_or((0.0, false), |q| (q, false))
        } else {
            let longest = explicit.iter().map(|(s, _)| *s).max().unwrap_or(1);
            let q = explicit
                .iter()
                .filter(|(s, _)| *s == longest)
                .map(|(_, q)| *q)
                .fold(0.0, f64::max);
            (q, true)
        };
        if q == 0.0 {
            continue;
        }
        let better = match &best {
            None => true,
            Some((blang, bq, bexplicit)) => {
                if (q, explicit_hit) != (*bq, *bexplicit) {
                    (q, explicit_hit) > (*bq, *bexplicit)
                } else {
                    *lang == DEFAULT_LOCALE && blang != DEFAULT_LOCALE
                }
            }
        };
        if better {
            best = Some(((*lang).to_string(), q, explicit_hit));
        }
    }
    best.map(|(lang, _, _)| lang)
}

/// 从请求上下文检测语言：Cookie（用户显式选择的记忆） > Accept-Language 协商 > 默认
/// 注意：`?lang=` 参数通道已废除；路径段是唯一权威语言入口
pub fn detect(cx: &Cx) -> String {
    if let Some(cookie) = topcoat::cookie::cookies(cx).get("lang") {
        if let Some(lang) = parse_locale(cookie.value())
            .as_ref()
            .and_then(resolve_supported)
        {
            return lang;
        }
    }
    let header = topcoat::router::request::headers(cx)
        .get("accept-language")
        .and_then(|v| v.to_str().ok());
    if let Some(negotiated) = header.and_then(negotiate_language) {
        remember(cx, &negotiated);
        return negotiated;
    }
    DEFAULT_LOCALE.to_string()
}

/// 服务端写入 lang cookie（语言记忆；切换语言、协商命中时调用）
pub fn remember(cx: &Cx, lang: &str) {
    use topcoat::cookie::{Cookies, cookie};
    topcoat::cookie::cookies(cx).add(cookie! {
        "lang" = lang.to_owned();
        Path = "/";
        SameSite = Lax;
        MaxAge = topcoat::cookie::time::Duration::days(365);
    });
}

/// 路径语言段归一化（全局层使用）：
/// 首段是真实语言标识但非规范或不受支持时，返回重写后的规范路径；
/// 其余情况（根路径、非语言段、已规范且受支持）返回 None 放行
pub fn normalize_locale_path(path: &str) -> Option<String> {
    let first = path.split('/').nth(1).unwrap_or("");
    if first.is_empty() {
        return None; // 根路径
    }
    if !is_real_locale(first) {
        return None; // 非语言段（assets、_topcoat、favicon.svg…）
    }
    let parsed = parse_locale(first).expect("is_real_locale 已确认可解析");
    let target = resolve_supported(&parsed).unwrap_or_else(|| DEFAULT_LOCALE.to_string());
    if target == first {
        return None; // 已是规范且受支持
    }
    let rest = path.split('/').skip(2).collect::<Vec<_>>().join("/");
    Some(if rest.is_empty() {
        format!("/{target}")
    } else {
        format!("/{target}/{rest}")
    })
}

/// 语言切换链接：重写路径首段为目标语言，保留其余路径与查询串；
/// 首段不是语言标识时插入语言段
pub fn swap_locale(path_and_query: &str, lang: &str) -> String {
    let (path, query) = path_and_query
        .split_once('?')
        .unwrap_or((path_and_query, ""));
    let first = path.split('/').nth(1).unwrap_or("");
    let swapped = if is_real_locale(first) {
        let rest = path.split('/').skip(2).collect::<Vec<_>>().join("/");
        if rest.is_empty() {
            format!("/{lang}")
        } else {
            format!("/{lang}/{rest}")
        }
    } else if path == "/" {
        format!("/{lang}")
    } else {
        format!("/{lang}{path}")
    };
    if query.is_empty() {
        swapped
    } else {
        format!("{swapped}?{query}")
    }
}

/// 语言菜单顺序：当前语言置顶，其余按字典序
pub fn menu_langs(current: &str) -> Vec<&'static str> {
    let mut langs: Vec<&'static str> = SUPPORTED.to_vec();
    if let Some(pos) = langs.iter().position(|l| *l == current) {
        let cur = langs.remove(pos);
        langs.insert(0, cur);
    }
    langs
}
