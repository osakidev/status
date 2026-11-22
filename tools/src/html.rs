//! このページの HTML を読むための小さな字句解析（osakidev/nokorazu-status の tools/src/html.rs から 2026-11-23 に写した。元は osakidev/site）。HTML の全部には対応しない。
//!
//! - タグ（開始・終了）と属性、文字の並びにする。コメントと `<!doctype>` は読み飛ばす
//! - 属性の値は `"…"` か `'…'` か、引用符なし。`&amp;` などのよく使う文字参照だけ戻す
//! - `<script>`・`<style>` の中身は文字として扱わない（このサイトでは使わない決まり。テストで断る）

/// タグ。名前・属性名は小文字にする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    /// `</name>`
    pub end: bool,
    /// `<name … />`（SVG の図形など）
    pub self_closing: bool,
    /// 何行目か（1 から）
    pub line: usize,
}

impl Tag {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Tag(Tag),
    Text(String),
}

/// 中身を持たない要素（終了タグを書かない）。
pub const VOID: [&str; 13] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr",
];

pub fn tokenize(src: &str) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    let mut rest = src;
    let line_of = |rest: &str| src[..src.len() - rest.len()].matches('\n').count() + 1;
    while !rest.is_empty() {
        let Some(lt) = rest.find('<') else {
            out.push(Token::Text(decode(rest)));
            break;
        };
        if lt > 0 {
            out.push(Token::Text(decode(&rest[..lt])));
        }
        rest = &rest[lt..];
        if let Some(after) = rest.strip_prefix("<!--") {
            let end = after
                .find("-->")
                .ok_or_else(|| format!("{} 行目: 閉じていないコメント", line_of(rest)))?;
            rest = &after[end + 3..];
            continue;
        }
        if rest.starts_with("<!") {
            let end = rest.find('>').ok_or("閉じていない <!")?;
            rest = &rest[end + 1..];
            continue;
        }
        let line = line_of(rest);
        let (tag, used) = parse_tag(rest).map_err(|e| format!("{line} 行目: {e}"))?;
        rest = &rest[used..];
        let raw = !tag.end && (tag.name == "script" || tag.name == "style");
        let name = tag.name.clone();
        out.push(Token::Tag(Tag { line, ..tag }));
        if raw {
            let close = format!("</{name}");
            let end = rest
                .to_ascii_lowercase()
                .find(&close)
                .ok_or_else(|| format!("{line} 行目: <{name}> が閉じていません"))?;
            rest = &rest[end..];
        }
    }
    Ok(out)
}

/// `<` から始まる 1 つのタグを読み、タグと使った長さ（バイト）を返す。
fn parse_tag(s: &str) -> Result<(Tag, usize), String> {
    let b = s.as_bytes();
    let mut i = 1;
    let end = b.get(i) == Some(&b'/');
    if end {
        i += 1;
    }
    let start = i;
    while b.get(i).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'-') {
        i += 1;
    }
    if i == start {
        return Err("タグの名前がありません（文字の < は &lt; と書く）".into());
    }
    let name = s[start..i].to_ascii_lowercase();
    let mut attrs = Vec::new();
    loop {
        while b.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        match b.get(i) {
            None => return Err(format!("<{name}> が閉じていません")),
            Some(b'>') => {
                return Ok((Tag { name, attrs, end, self_closing: false, line: 0 }, i + 1));
            }
            Some(b'/') if b.get(i + 1) == Some(&b'>') => {
                return Ok((Tag { name, attrs, end, self_closing: true, line: 0 }, i + 2));
            }
            _ => {}
        }
        let a0 = i;
        while b.get(i).is_some_and(|c| !c.is_ascii_whitespace() && !b"=>/\"'".contains(c)) {
            i += 1;
        }
        if i == a0 {
            return Err(format!("<{name}> の属性が読めません"));
        }
        let attr = s[a0..i].to_ascii_lowercase();
        let mut value = String::new();
        if b.get(i) == Some(&b'=') {
            i += 1;
            match b.get(i) {
                Some(&q) if q == b'"' || q == b'\'' => {
                    let close = s[i + 1..]
                        .find(q as char)
                        .ok_or_else(|| format!("<{name}> の {attr} の引用符が閉じていません"))?;
                    value = decode(&s[i + 1..i + 1 + close]);
                    i += close + 2;
                }
                _ => {
                    let v0 = i;
                    while b.get(i).is_some_and(|c| !c.is_ascii_whitespace() && *c != b'>') {
                        i += 1;
                    }
                    value = decode(&s[v0..i]);
                }
            }
        }
        if attrs.iter().any(|(n, _)| *n == attr) {
            return Err(format!("<{name}> の属性 {attr} が 2 つあります"));
        }
        attrs.push((attr, value));
    }
}

/// よく使う文字参照だけを戻す（`&amp;` `&lt;` `&gt;` `&quot;` `&#39;` `&nbsp;`、数値参照）。
pub fn decode(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let Some(semi) = rest.find(';').filter(|&n| n <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..semi];
        let ch = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            _ => ent
                .strip_prefix("#x")
                .or_else(|| ent.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| ent.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// 読み上げ・表示される文字（タグの外の文字。`<title>` を含む）をつなげる。
pub fn text(tokens: &[Token]) -> String {
    let mut s = String::new();
    for t in tokens {
        if let Token::Text(x) = t {
            s.push_str(x);
        }
    }
    s
}

/// タグだけを順に返す。
pub fn tags(tokens: &[Token]) -> impl Iterator<Item = &Tag> {
    tokens.iter().filter_map(|t| match t {
        Token::Tag(tag) => Some(tag),
        Token::Text(_) => None,
    })
}

/// 開始タグと終了タグの対応を確かめる（中身を持たない要素を除く）。
pub fn check_nesting(tokens: &[Token]) -> Result<(), String> {
    let mut stack: Vec<&Tag> = Vec::new();
    for tag in tags(tokens) {
        if VOID.contains(&tag.name.as_str()) {
            if tag.end {
                return Err(format!("{} 行目: </{}> は書かない", tag.line, tag.name));
            }
            continue;
        }
        if !tag.end {
            // SVG の図形（rect・circle）は <… /> で閉じる
            if tag.self_closing {
                continue;
            }
            stack.push(tag);
            continue;
        }
        match stack.pop() {
            Some(open) if open.name == tag.name => {}
            Some(open) => {
                return Err(format!(
                    "{} 行目: </{}> の前に、{} 行目の <{}> が閉じていません",
                    tag.line, tag.name, open.line, open.name
                ));
            }
            None => return Err(format!("{} 行目: 対応する開始タグのない </{}>", tag.line, tag.name)),
        }
    }
    match stack.pop() {
        Some(open) => Err(format!("{} 行目の <{}> が閉じていません", open.line, open.name)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn タグと属性() {
        let t = tokenize("<!doctype html>\n<!-- x -->\n<a href=\"/a/?x=1&amp;y=2\" class='c' hidden data-x=1>リンク</a>").unwrap();
        let a = tags(&t).next().unwrap();
        assert_eq!(a.name, "a");
        assert_eq!(a.line, 3);
        assert_eq!(a.attr("href"), Some("/a/?x=1&y=2"));
        assert_eq!(a.attr("class"), Some("c"));
        assert_eq!(a.attr("hidden"), Some(""));
        assert_eq!(a.attr("data-x"), Some("1"));
        assert_eq!(text(&t), "\n\nリンク");
        assert!(check_nesting(&t).is_ok());
    }

    #[test]
    fn 文字参照() {
        assert_eq!(decode("a&lt;b&gt;&quot;&#39;&#x41;&nbsp;"), "a<b>\"'A\u{a0}");
        assert_eq!(decode("AT&T &unknown; &"), "AT&T &unknown; &");
    }

    #[test]
    fn 対応の誤り() {
        assert!(check_nesting(&tokenize("<p><a></p></a>").unwrap()).is_err());
        assert!(check_nesting(&tokenize("<div>").unwrap()).is_err());
        assert!(check_nesting(&tokenize("</div>").unwrap()).is_err());
        assert!(check_nesting(&tokenize("<meta charset=\"utf-8\"><br><p>x</p>").unwrap()).is_ok());
        assert!(check_nesting(&tokenize("<svg><rect x=\"1\"/><circle r=\"1\"/></svg>").unwrap()).is_ok());
    }

    #[test]
    fn 読めない_html() {
        assert!(tokenize("<a href=\"x>").is_err());
        assert!(tokenize("<!-- x").is_err());
        assert!(tokenize("a < b").is_err());
        assert!(tokenize("<a x=1 x=2>").is_err());
        assert!(tokenize("<script>x").is_err());
    }

    #[test]
    fn スクリプトの中身は文字にしない() {
        let t = tokenize("<p>a</p><script>if (a < b) {}</script><p>b</p>").unwrap();
        assert_eq!(text(&t), "ab");
    }
}
