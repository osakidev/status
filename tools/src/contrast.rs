//! WCAG 2.2 の相対輝度とコントラスト比（ガイドライン 4.2 の「計算」。osakidev/nokorazu-status の tools/src/contrast.rs から 2026-11-23 に写した。元は osakidev/site）。
//!
//! sRGB の各成分を 0.04045 を境に線形化し、`L = 0.2126R + 0.7152G + 0.0722B`、
//! 比 = `(L1 + 0.05) / (L2 + 0.05)`（L1 は明るい方）。比は丸めない（4.499 は 4.5 を満たさない）。

/// `#rgb` か `#rrggbb`（大文字・小文字は問わない）を読む。それ以外は `None`。
pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let hex = s.trim().strip_prefix('#')?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let v = |i: usize, n: usize| u8::from_str_radix(hex.get(i..i + n)?, 16).ok();
    match hex.len() {
        3 => Some([v(0, 1)? * 17, v(1, 1)? * 17, v(2, 1)? * 17]),
        6 => Some([v(0, 2)?, v(2, 2)?, v(4, 2)?]),
        _ => None,
    }
}

/// 相対輝度（0〜1）。
pub fn luminance(rgb: [u8; 3]) -> f64 {
    let lin = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(rgb[0]) + 0.7152 * lin(rgb[1]) + 0.0722 * lin(rgb[2])
}

/// コントラスト比（1〜21）。順番は問わない。
pub fn ratio(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// 表に書くための値。小数第 2 位で切り捨てる（ガイドライン 4.2。丸めて基準を満たしたように見せない）。
pub fn floor2(r: f64) -> f64 {
    (r * 100.0).floor() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 色を読む() {
        assert_eq!(parse_hex("#fff"), Some([255, 255, 255]));
        assert_eq!(parse_hex("#1D1d1f"), Some([0x1d, 0x1d, 0x1f]));
        assert_eq!(parse_hex(" #000000 "), Some([0, 0, 0]));
        for bad in ["fff", "#ffff", "#12345", "#gggggg", "#", "", "#ffffff00", "#+1+2+3"] {
            assert_eq!(parse_hex(bad), None, "{bad}");
        }
    }

    #[test]
    fn 白と黒は21() {
        assert_eq!(ratio([0, 0, 0], [255, 255, 255]), 21.0);
        assert_eq!(ratio([255, 255, 255], [255, 255, 255]), 1.0);
        assert_eq!(ratio([255, 255, 255], [0, 0, 0]), 21.0);
    }

    /// ガイドライン 4.4 の表の値（切り捨て）と一致する
    #[test]
    fn ガイドラインの表と同じ値() {
        let c = |s| parse_hex(s).unwrap();
        let cases = [
            ("#1d1d1f", "#f7f6f2", 15.56),
            ("#5a5a60", "#f7f6f2", 6.33),
            ("#767570", "#ffffff", 4.61),
            ("#1f4e8c", "#f7f6f2", 7.68),
            ("#ececec", "#1b1c1e", 14.43),
            ("#a8a8ae", "#25262a", 6.38),
            ("#85878e", "#25262a", 4.21),
            ("#8db4ec", "#1b1c1e", 8.01),
            ("#c9c7c0", "#f7f6f2", 1.56),
        ];
        for (fg, bg, want) in cases {
            assert_eq!(floor2(ratio(c(fg), c(bg))), want, "{fg} / {bg}");
        }
    }

    #[test]
    fn 切り捨て() {
        assert_eq!(floor2(4.4999), 4.49);
        assert_eq!(floor2(4.5), 4.5);
    }
}
