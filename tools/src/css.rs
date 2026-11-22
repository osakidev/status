//! style.css を読むための小さな解析（osakidev/nokorazu-status の tools/src/css.rs から 2026-11-23 に写した。元は osakidev/site）。自分で書いた CSS が読めればよく、CSS の全部には対応しない。
//!
//! - コメントを除き、規則（セレクター＋宣言）と、それを囲む at 規則（`@media` など）の並びにする
//! - 色の変数は `:root` にだけ書く決まり。通常（ライト）・`@media (prefers-color-scheme: dark)`・`@media print`
//!   の 3 つの `:root` から、テーマごとの値を作る

use std::collections::BTreeMap;

/// 宣言（`color: var(--fg)` など）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decl {
    pub prop: String,
    pub value: String,
}

/// 規則。`at` は外側から順の at 規則の前置き（`@media (prefers-color-scheme: dark)` など）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub at: Vec<String>,
    pub selector: String,
    pub decls: Vec<Decl>,
}

/// 解析の結果。`statements` は `;` で終わる at 規則（`@import` など）。
#[derive(Debug, Default)]
pub struct Sheet {
    pub rules: Vec<Rule>,
    pub statements: Vec<String>,
}

/// 中に規則を持つ at 規則（それ以外の at 規則は、宣言を持つ規則として扱う）。
const GROUPING: [&str; 4] = ["@media", "@supports", "@layer", "@container"];

pub fn parse(css: &str) -> Result<Sheet, String> {
    let text = strip_comments(css)?;
    let chars: Vec<char> = text.chars().collect();
    let mut sheet = Sheet::default();
    let mut pos = 0;
    parse_rules(&chars, &mut pos, &[], &mut sheet, false)?;
    Ok(sheet)
}

/// `/* */` を除く。文字列（`"…"`・`'…'`）の中は除かない。
pub fn strip_comments(css: &str) -> Result<String, String> {
    let mut out = String::with_capacity(css.len());
    let mut it = css.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(c) = it.next() {
        if let Some(q) = quote {
            out.push(c);
            if c == '\\' {
                if let Some(n) = it.next() {
                    out.push(n);
                }
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                quote = Some(c);
                out.push(c);
            }
            '/' if it.peek() == Some(&'*') => {
                it.next();
                let mut prev = '\0';
                loop {
                    match it.next() {
                        Some('/') if prev == '*' => break,
                        Some(n) => prev = n,
                        None => return Err("閉じていないコメントがあります".into()),
                    }
                }
                out.push(' ');
            }
            _ => out.push(c),
        }
    }
    if quote.is_some() {
        return Err("閉じていない文字列があります".into());
    }
    Ok(out)
}

/// `stop` の文字のどれか（文字列・括弧の外）まで読み、読んだ文字列と止まった文字を返す。
fn read_until(chars: &[char], pos: &mut usize, stop: &[char]) -> (String, Option<char>) {
    let mut s = String::new();
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    while let Some(&c) = chars.get(*pos) {
        *pos += 1;
        if let Some(q) = quote {
            s.push(c);
            if c == '\\' {
                if let Some(&n) = chars.get(*pos) {
                    s.push(n);
                    *pos += 1;
                }
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 && stop.contains(&c) => return (s, Some(c)),
            _ => {}
        }
        s.push(c);
    }
    (s, None)
}

fn parse_rules(
    chars: &[char],
    pos: &mut usize,
    at: &[String],
    sheet: &mut Sheet,
    nested: bool,
) -> Result<(), String> {
    loop {
        let (prelude, stop) = read_until(chars, pos, &['{', ';', '}']);
        let prelude = collapse(&prelude);
        match stop {
            None => {
                if nested {
                    return Err(format!("閉じていない {{ があります（{}）", at.join(" ")));
                }
                if !prelude.is_empty() {
                    return Err(format!("{{ のない記述があります: {prelude}"));
                }
                return Ok(());
            }
            Some('}') => {
                if !nested {
                    return Err("対応する { のない } があります".into());
                }
                if !prelude.is_empty() {
                    return Err(format!("宣言の外に記述があります: {prelude}"));
                }
                return Ok(());
            }
            Some(';') => {
                if prelude.is_empty() {
                    continue;
                }
                if !prelude.starts_with('@') {
                    return Err(format!("規則の外に宣言があります: {prelude}"));
                }
                sheet.statements.push(prelude);
            }
            Some(_) => {
                if GROUPING.iter().any(|g| prelude.starts_with(g)) {
                    let mut inner = at.to_vec();
                    inner.push(prelude);
                    parse_rules(chars, pos, &inner, sheet, true)?;
                } else {
                    let (body, stop) = read_until(chars, pos, &['}', '{']);
                    if stop != Some('}') {
                        return Err(format!("「{prelude}」の宣言が閉じていません"));
                    }
                    sheet.rules.push(Rule {
                        at: at.to_vec(),
                        selector: prelude,
                        decls: parse_decls(&body)?,
                    });
                }
            }
        }
    }
}

fn parse_decls(body: &str) -> Result<Vec<Decl>, String> {
    let chars: Vec<char> = body.chars().collect();
    let mut pos = 0;
    let mut decls = Vec::new();
    loop {
        let (item, stop) = read_until(&chars, &mut pos, &[';']);
        let item = item.trim();
        if !item.is_empty() {
            let Some((prop, value)) = item.split_once(':') else {
                return Err(format!("「名前: 値」の形ではありません: {item}"));
            };
            decls.push(Decl {
                prop: prop.trim().to_string(),
                value: collapse(value),
            });
        }
        if stop.is_none() {
            return Ok(decls);
        }
    }
}

/// 空白の並びを 1 つの空白にし、前後の空白を除く。
fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// テーマごとの色の変数（名前 → `#rrggbb`）。
#[derive(Debug, Default)]
pub struct Themes {
    /// 通常（ライト）
    pub light: BTreeMap<String, String>,
    /// ダークで置き直す変数だけ
    pub dark_only: BTreeMap<String, String>,
    /// 印刷で置き直す変数だけ
    pub print_only: BTreeMap<String, String>,
}

impl Themes {
    /// ダーク表示で使われる値（ライトにダークの分を重ねたもの）。
    pub fn dark(&self) -> BTreeMap<String, String> {
        overlay(&self.light, &self.dark_only)
    }

    /// 印刷で使われる値。端末がダーク表示のときも印刷の規則が後に来るので、ダークの値に印刷の分が重なる。
    pub fn print(&self) -> BTreeMap<String, String> {
        overlay(&self.dark(), &self.print_only)
    }
}

fn overlay(base: &BTreeMap<String, String>, top: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut m = base.clone();
    m.extend(top.iter().map(|(k, v)| (k.clone(), v.clone())));
    m
}

/// 色の値として読めるか（`#rgb` / `#rrggbb`）。
fn is_hex_color(v: &str) -> bool {
    crate::contrast::parse_hex(v).is_some()
}

/// `:root` の色の変数をテーマごとに集める。決まりから外れる書き方（ほかの場所の `:root`、
/// `#rrggbb` 以外の色の変数）は誤りにする。
pub fn color_tokens(sheet: &Sheet) -> Result<Themes, String> {
    let mut t = Themes::default();
    for rule in sheet.rules.iter().filter(|r| r.selector == ":root") {
        let target = match rule.at.as_slice() {
            [] => &mut t.light,
            [m] if m == "@media (prefers-color-scheme: dark)" => &mut t.dark_only,
            [m] if m == "@media print" => &mut t.print_only,
            other => return Err(format!(":root を想定していない場所に書いています: {other:?}")),
        };
        for d in rule.decls.iter().filter(|d| d.prop.starts_with("--")) {
            if is_hex_color(&d.value) {
                target.insert(d.prop.clone(), d.value.to_ascii_lowercase());
            } else if !color_literals(&d.value, false).is_empty() {
                return Err(format!("色の変数 {} は #rrggbb で書いてください: {}", d.prop, d.value));
            }
        }
    }
    Ok(t)
}

/// 強制カラーのときだけ使ってよいシステムの色。
const SYSTEM_COLORS: [&str; 19] = [
    "accentcolor",
    "accentcolortext",
    "activetext",
    "buttonborder",
    "buttonface",
    "buttontext",
    "canvas",
    "canvastext",
    "field",
    "fieldtext",
    "graytext",
    "highlight",
    "highlighttext",
    "linktext",
    "mark",
    "marktext",
    "selecteditem",
    "selecteditemtext",
    "visitedtext",
];

/// よく使われる色の名前（CSS の名前付きの色のうち、うっかり書きそうなもの）。
const NAMED_COLORS: [&str; 24] = [
    "black", "white", "red", "green", "blue", "yellow", "orange", "purple", "gray", "grey", "silver",
    "navy", "maroon", "olive", "teal", "aqua", "fuchsia", "lime", "pink", "brown", "gold", "cyan",
    "magenta", "indigo",
];

const COLOR_FUNCTIONS: [&str; 11] = [
    "rgb(", "rgba(", "hsl(", "hsla(", "hwb(", "lab(", "lch(", "oklab(", "oklch(", "color(", "color-mix(",
];

/// 値の中の色の書き方（`#…`、`rgb(…)` など、色の名前）を返す。`allow_system` なら強制カラーのシステムの色は除く。
/// 文字列（`"…"`）の中は見ない。
pub fn color_literals(value: &str, allow_system: bool) -> Vec<String> {
    let mut found = Vec::new();
    let mut outside = String::new();
    let mut quote: Option<char> = None;
    for c in value.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                outside.push(' ');
            }
            None => outside.push(c),
        }
    }
    let lower = outside.to_ascii_lowercase();
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '-';
    for f in COLOR_FUNCTIONS {
        for (i, _) in lower.match_indices(f) {
            if !lower[..i].chars().next_back().is_some_and(is_ident) {
                found.push(f.to_string());
            }
        }
    }
    for word in lower.split(|c: char| !(is_ident(c) || c == '#')) {
        if (word.starts_with('#') && word.len() > 1)
            || NAMED_COLORS.contains(&word)
            || (SYSTEM_COLORS.contains(&word) && !allow_system)
        {
            found.push(word.to_string());
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
/* コメント { } */
:root { color-scheme: light dark; --bg: #F7F6F2; --fg: #1d1d1f; --font: "A;B", sans-serif; }
@media (prefers-color-scheme: dark) {
  :root { --bg: #1b1c1e; }
}
body { background: var(--bg); color: var(--fg); }
a::after { content: "（" attr(href) "）"; }
@media print { :root { --fg: #000; } .x { display: none } }
"#;

    #[test]
    fn 規則と宣言() {
        let s = parse(SAMPLE).unwrap();
        let sels: Vec<_> = s.rules.iter().map(|r| r.selector.as_str()).collect();
        assert_eq!(sels, [":root", ":root", "body", "a::after", ":root", ".x"]);
        assert_eq!(s.rules[1].at, ["@media (prefers-color-scheme: dark)"]);
        assert_eq!(s.rules[0].decls[3].value, "\"A;B\", sans-serif");
        assert_eq!(s.rules[3].decls[0].value, "\"（\" attr(href) \"）\"");
        assert_eq!(s.rules[5].decls[0], Decl { prop: "display".into(), value: "none".into() });
    }

    #[test]
    fn テーマごとの色() {
        let t = color_tokens(&parse(SAMPLE).unwrap()).unwrap();
        assert_eq!(t.light["--bg"], "#f7f6f2");
        assert_eq!(t.dark()["--bg"], "#1b1c1e");
        assert_eq!(t.dark()["--fg"], "#1d1d1f");
        assert_eq!(t.print()["--fg"], "#000");
        assert!(!t.light.contains_key("--font"));
    }

    #[test]
    fn 壊れた_css() {
        assert!(parse("a { color: red").is_err());
        assert!(parse("a { color: red } }").is_err());
        assert!(parse("/* x").is_err());
        assert!(parse("color: red;").is_err());
        assert!(parse("@media print { a { x: y }").is_err());
        assert!(color_tokens(&parse(":root { --x: rgb(1 2 3) }").unwrap()).is_err());
        assert!(color_tokens(&parse("@media (min-width: 1px) { :root { --x: #fff } }").unwrap()).is_err());
    }

    #[test]
    fn 色の書き方を見つける() {
        assert_eq!(color_literals("1px solid #fff", false), ["#fff"]);
        assert_eq!(color_literals("rgb(0 0 0 / 50%)", false), ["rgb("]);
        assert_eq!(color_literals("2px dashed Red", false), ["red"]);
        assert_eq!(color_literals("CanvasText", false), ["canvastext"]);
        assert!(color_literals("CanvasText", true).is_empty());
        assert!(color_literals("var(--fg)", false).is_empty());
        assert!(color_literals("1px solid currentColor", false).is_empty());
        assert!(color_literals("\"Hiragino Sans\", sans-serif", false).is_empty());
        assert!(color_literals("\"#fff\"", false).is_empty());
        assert!(color_literals("clamp(2.5rem, 1.25rem + 5.5vw, 6.5rem)", false).is_empty());
    }
}
