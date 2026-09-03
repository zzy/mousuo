/// 将「分」单位的金额格式化为美元字符串，如 1299 → "$12.99"
///
/// 全程整数运算手动拼接，不使用 f64，避免浮点误差。
pub fn format_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    let dollars = abs / 100;
    let remainder = abs % 100;
    format!("{sign}${dollars}.{remainder:02}")
}
