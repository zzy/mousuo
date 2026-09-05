//! 数学算式验证码 — 生成 SVG，答案存入当前 session
//!
//! 生成时调用 generate()，答案通过 save_answer() 写入 session；
//! 验证时从 session 读取答案比对，无需 HMAC 签名。

use std::time::{SystemTime, UNIX_EPOCH};

use topcoat::context::Cx;

use crate::common::constant::CAPTCHA_EXPIRY;
use crate::common::session;
use crate::db;

pub struct Captcha {
    pub svg: String,
    pub answer: u8,
}

/// 生成验证码（纯函数，无副作用）
pub fn generate() -> Captcha {
    let a = gen_i32(1, 9) as u8;
    let b = gen_i32(1, 9) as u8;
    let op = if gen_bool() { '+' } else { '-' };
    let answer = match op {
        '+' => a + b,
        '-' => {
            if a >= b {
                a - b
            } else {
                b - a
            }
        }
        _ => unreachable!(),
    };
    let (left, right) = if op == '-' && a < b { (b, a) } else { (a, b) };
    let svg = render_svg(left, right, op);
    Captcha { svg, answer }
}

/// 将验证码答案写入当前 session
pub async fn save_answer(cx: &Cx, answer: u8) -> Result<(), String> {
    let Some(hash) = topcoat::session::token_hash(cx)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    let id = session::encode_id(&hash);
    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs()
        + CAPTCHA_EXPIRY;
    let db = db::get_db();
    // 登录前的会话尚无记录：UPDATE 命中 0 行时必须补建，否则验证码无处存放
    let mut probe = db
        .query("SELECT id FROM session WHERE id = type::record('session', $id)")
        .bind(("id", id.clone()))
        .await
        .map_err(|e| e.to_string())?;
    let rows: Vec<surrealdb::types::Value> = probe.take(0).map_err(|e| e.to_string())?;
    let sql = if rows.is_empty() {
        "CREATE session CONTENT { id: type::record('session', $id), username: '', expires_at: $expires_at, captcha_answer: $answer, captcha_expires_at: $expires_at }"
    } else {
        "UPDATE session SET captcha_answer = $answer, captcha_expires_at = $expires_at WHERE id = type::record('session', $id)"
    };
    db.query(sql)
        .bind(("id", id))
        .bind(("answer", answer as i64))
        .bind(("expires_at", expires_at as i64))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 验证用户输入：从 session 读取答案比对，通过后清除
pub async fn verify(cx: &Cx, user_answer: &str) -> bool {
    let Ok(Some(hash)) = topcoat::session::token_hash(cx).await else {
        return false;
    };
    let id = session::encode_id(&hash);
    let db = db::get_db();
    let mut res = match db
        .query("SELECT captcha_answer, captcha_expires_at FROM session WHERE id = type::record('session', $id)")
        .bind(("id", id.clone()))
        .await
    {
        Ok(r) => r,
        Err(_) => return false,
    };
    let rows: Vec<surrealdb::types::Value> = match res.take(0) {
        Ok(r) => r,
        Err(_) => return false,
    };
    // 过期比较在 Rust 侧完成（均为 unix 秒整数，避免服务端类型比较歧义）
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let correct = match rows.first().and_then(|v| v.as_object()) {
        Some(obj) => {
            let stored_answer = obj
                .get("captcha_answer")
                .and_then(|v| db::from_value::<i64>(v));
            let stored_expiry = obj
                .get("captcha_expires_at")
                .and_then(|v| db::from_value::<i64>(v))
                .unwrap_or(0);
            match stored_answer {
                Some(ans) => {
                    user_answer
                        .parse::<u8>()
                        .map_or(false, |user| user == ans as u8 && stored_expiry > now)
                }
                None => false,
            }
        }
        None => false,
    };
    // 无论对错，清除已使用的验证码
    let _ = db
        .query("UPDATE type::record('session', $id) SET captcha_answer = NONE, captcha_expires_at = NONE")
        .bind(("id", id))
        .await;
    correct
}

// ── 随机数 / SVG 渲染 ─────────────────────────────────────────

fn gen_i32(min: i32, max: i32) -> i32 {
    let mut buf = [0u8; 4];
    aws_lc_rs::rand::fill(&mut buf).unwrap();
    min + (u32::from_le_bytes(buf) % ((max - min + 1) as u32)) as i32
}

fn gen_f64(min: f64, max: f64) -> f64 {
    let mut buf = [0u8; 8];
    aws_lc_rs::rand::fill(&mut buf).unwrap();
    min + (u64::from_le_bytes(buf) as f64 / u64::MAX as f64) * (max - min)
}

fn gen_bool() -> bool {
    let mut buf = [0u8; 1];
    aws_lc_rs::rand::fill(&mut buf).unwrap();
    buf[0] & 1 == 0
}

fn render_svg(left: u8, right: u8, op: char) -> String {
    let w: f64 = 160.0;
    let h: f64 = 40.0;
    let cy: f64 = 26.0;
    let base = 18.0;
    let cx1 = 22.0 + gen_f64(-4.0, 4.0);
    let cx2 = 58.0 + gen_f64(-4.0, 4.0);
    let cx3 = 92.0 + gen_f64(-4.0, 4.0);
    let s1 = base + gen_f64(-2.0, 2.0);
    let s2 = base + gen_f64(-2.0, 2.0);
    let s3 = base + gen_f64(-2.0, 2.0);
    let r1 = gen_f64(-20.0, 20.0);
    let r2 = gen_f64(-20.0, 20.0);
    let r3 = gen_f64(-20.0, 20.0);
    let dy1 = gen_f64(-4.0, 4.0);
    let dy2 = gen_f64(-4.0, 4.0);
    let dy3 = gen_f64(-4.0, 4.0);
    let hue = gen_i32(200, 260);
    let c1 = format!("hsl({hue}, {}%, {}%)", gen_i32(45, 65), gen_i32(25, 40));
    let c2 = format!(
        "hsl({}, {}%, {}%)",
        gen_i32(200, 260),
        gen_i32(45, 65),
        gen_i32(25, 40)
    );

    let mut buf = String::new();
    let p = |s: &mut String, t: &str| s.push_str(t);

    p(
        &mut buf,
        &format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">"##
        ),
    );
    p(
        &mut buf,
        &format!(r##"<rect width="{w}" height="{h}" rx="4" fill="#fafbfc"/>"##),
    );
    for i in 1..3 {
        let gx = i as f64 * 53.0;
        p(
            &mut buf,
            &format!(
                r##"<line x1="{gx}" y1="2" x2="{gx}" y2="{h}" stroke="#e9ecef" stroke-width="0.5"/>"##
            ),
        );
    }
    for _ in 0..5 {
        let lx = gen_f64(0.0, w);
        let ly = gen_f64(5.0, h - 5.0);
        let lx2 = lx + gen_f64(-60.0, 60.0);
        let ly2 = ly + gen_f64(-25.0, 25.0);
        let sw = gen_f64(0.3, 0.8);
        let op = gen_f64(0.15, 0.35);
        p(
            &mut buf,
            &format!(
                r##"<line x1="{lx}" y1="{ly}" x2="{lx2}" y2="{ly2}" stroke="#adb5bd" stroke-width="{sw}" opacity="{op}"/>"##
            ),
        );
    }
    for _ in 0..4 {
        let lx = gen_f64(15.0, 110.0);
        let ly = gen_f64(8.0, h - 8.0);
        let lx2 = lx + gen_f64(-30.0, 30.0);
        let ly2 = ly + gen_f64(-15.0, 15.0);
        p(
            &mut buf,
            &format!(
                r##"<line x1="{lx}" y1="{ly}" x2="{lx2}" y2="{ly2}" stroke="#adb5bd" stroke-width="0.6" opacity="0.35"/>"##
            ),
        );
    }
    for _ in 0..25 {
        let dx = gen_f64(8.0, w - 8.0);
        let dy = gen_f64(6.0, h - 6.0);
        let dr = gen_f64(0.3, 1.5);
        p(
            &mut buf,
            &format!(r##"<circle cx="{dx}" cy="{dy}" r="{dr}" fill="#6c757d" opacity="0.25"/>"##),
        );
    }
    for _ in 0..4 {
        let fd = gen_i32(0, 9);
        let fx = gen_f64(8.0, w - 8.0);
        let fy = gen_f64(8.0, h - 6.0);
        let fs = gen_f64(6.0, 10.0);
        let fr = gen_f64(-30.0, 30.0);
        p(
            &mut buf,
            &format!(
                r##"<text x="{fx}" y="{fy}" font-size="{fs}" font-family="Courier,monospace" fill="#ced4da" text-anchor="middle" transform="rotate({fr} {fx} {fy})">{fd}</text>"##
            ),
        );
    }
    p(
        &mut buf,
        &format!(
            r##"<text x="{cx1}" y="{cy}" dy="{dy1}" font-size="{s1}" font-family="Courier,monospace" font-weight="bold" fill="{c1}" text-anchor="middle" transform="rotate({r1} {cx1} {cy})">{left}</text>"##
        ),
    );
    p(
        &mut buf,
        &format!(
            r##"<text x="{cx2}" y="{cy}" dy="{dy2}" font-size="{s2}" font-family="Courier,monospace" font-weight="bold" fill="#e03131" text-anchor="middle" transform="rotate({r2} {cx2} {cy})">{op}</text>"##
        ),
    );
    p(
        &mut buf,
        &format!(
            r##"<text x="{cx3}" y="{cy}" dy="{dy3}" font-size="{s3}" font-family="Courier,monospace" font-weight="bold" fill="{c2}" text-anchor="middle" transform="rotate({r3} {cx3} {cy})">{right}</text>"##
        ),
    );
    p(
        &mut buf,
        &format!(
            r##"<text x="130" y="{cy}" font-size="{base}" font-family="Courier,monospace" font-weight="bold" fill="#495057" text-anchor="middle">=</text>"##
        ),
    );
    for _ in 0..8 {
        let ox = gen_f64(15.0, 110.0);
        let oy = gen_f64(8.0, h - 8.0);
        let or_ = gen_f64(0.8, 2.5);
        p(
            &mut buf,
            &format!(r##"<circle cx="{ox}" cy="{oy}" r="{or_}" fill="#6c757d" opacity="0.45"/>"##),
        );
    }
    for _ in 0..5 {
        let ox = gen_f64(15.0, 110.0);
        let oy = gen_f64(8.0, h - 8.0);
        let ox2 = ox + gen_f64(-10.0, 10.0);
        let oy2 = oy + gen_f64(-4.0, 4.0);
        p(
            &mut buf,
            &format!(
                r##"<line x1="{ox}" y1="{oy}" x2="{ox2}" y2="{oy2}" stroke="#6c757d" stroke-width="1.4" stroke-linecap="round" opacity="0.40"/>"##
            ),
        );
    }
    p(&mut buf, "</svg>");
    buf
}
