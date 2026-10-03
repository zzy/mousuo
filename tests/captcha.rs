//! 验证码生成纯函数测试（答案与 SVG 表达式一致性）

use mousuo::common::captcha;

/// 从 SVG 提取主表达式文本（左操作数/运算符/右操作数/=）
/// 渲染顺序：噪点在前，表达式四个文本在最后
fn expression_texts(svg: &str) -> Vec<String> {
    let mut texts: Vec<String> = svg
        .split("</text>")
        .filter_map(|s| {
            let idx = s.rfind('>')?;
            let t = s[idx + 1..].trim();
            (!t.is_empty()).then(|| t.to_string())
        })
        .collect();
    texts.split_off(texts.len().saturating_sub(4))
}

#[test]
fn answer_matches_svg_expression() {
    for _ in 0..100 {
        let cap = captcha::generate();
        let texts = expression_texts(&cap.svg);
        assert_eq!(texts.len(), 4, "svg 应含左/运算符/右/等号四个表达式文本");
        let left: u8 = texts[0].parse().expect("左操作数应为数字");
        let op = texts[1].as_str();
        let right: u8 = texts[2].parse().expect("右操作数应为数字");
        assert_eq!(texts[3], "=");
        let expected = match op {
            "+" => left + right,
            "-" => left - right, // 生成时保证左 ≥ 右
            _ => panic!("未知运算符: {op}"),
        };
        assert_eq!(cap.answer, expected, "svg 表达式 {left}{op}{right} 与答案不符");
    }
}

#[test]
fn operands_within_range_and_svg_wellformed() {
    for _ in 0..50 {
        let cap = captcha::generate();
        let texts = expression_texts(&cap.svg);
        let left: u8 = texts[0].parse().unwrap();
        let right: u8 = texts[2].parse().unwrap();
        assert!((1..=9).contains(&left) && (1..=9).contains(&right));
        assert!(cap.svg.starts_with("<svg"));
        assert!(cap.svg.contains("viewBox=\"0 0 160 40\""));
        assert!(cap.svg.trim_end().ends_with("</svg>"));
        assert!(cap.answer <= 18);
    }
}
