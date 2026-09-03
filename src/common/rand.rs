//! 安全随机工具 — aws_lc_rs 随机源
//!
//! CSRF token、账户激活 token 等安全凭据统一从此生成

/// 字节 → 小写 hex（64 位 / 32 字节等场景共用）
pub fn hex(bytes: &[u8]) -> String {
    const TABLE: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(TABLE[(b >> 4) as usize] as char);
        out.push(TABLE[(b & 0x0f) as usize] as char);
    }
    out
}

/// 32 字节安全随机 → 64 位小写 hex
pub fn random_hex() -> String {
    let mut buf = [0u8; 32];
    aws_lc_rs::rand::fill(&mut buf).expect("随机源不可用");
    hex(&buf)
}

/// 恒定时间比较（防时序侧信道；长度不等直接 false）
pub fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}
