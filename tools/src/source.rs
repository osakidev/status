//! サービスの一覧（`services.md`）を読み、決まりどおりか確かめる。決まりは README の「3. サービスを足すとき」。
//!
//! ```text
//! # OSAKI Dev のサービスの一覧        ← 先頭の題（任意）。公開しない
//! > 書き方の覚え書き                  ← `>` の行（任意）。公開しない
//!
//! ## nokorazu                         ← 1 つのサービスは `## 名前` の行から
//! 説明: ……するサービスです。
//! 稼働状況: https://status.nokorazu.jp/
//! ```
//!
//! 並びはページの並び。このページはサービスの状態を書かない（状態はサービスごとの稼働状況のページが持つ）。
//! 誤りは全部集めて、行の番号と直し方つきで返す。誤りがあれば組み立てない（急いで載せるものがないので、注意で済ませない）。

use std::path::Path;

/// 一覧のファイルの名前（リポジトリ直下）。
pub const FILE: &str = "services.md";
/// 書かない言葉（company の docs/design/guidelines.md 3.5・13.。法人化していないので「株式会社」も書かない）。
/// サービスは「道具」と呼ばない（社内の言葉の決まり）。
pub const AVOID_WORDS: [&str; 7] = ["安全", "完全", "100%", "100％", "絶対", "株式会社", "道具"];
/// 名前と説明の長さの上限（字）。一覧で 1〜2 行に収める。
pub const MAX_NAME: usize = 30;
pub const MAX_DESC: usize = 80;

/// サービス 1 つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    /// 表示の名前（`nokorazu`）
    pub name: String,
    /// 短い説明（1 文。「。」で終わる）
    pub desc: String,
    /// そのサービスの稼働状況のページ（`https://status.nokorazu.jp/`）
    pub url: String,
}

impl Service {
    /// リンクの文に添える住所（`status.nokorazu.jp`）。`https://` と最後の `/` を除いたもの。
    pub fn address(&self) -> &str {
        let s = self.url.strip_prefix("https://").unwrap_or(&self.url);
        s.strip_suffix('/').unwrap_or(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    pub services: Vec<Service>,
}

/// `dir` の `services.md` を読む。
pub fn load(dir: &Path) -> Result<Content, Vec<String>> {
    let path = dir.join(FILE);
    let text = std::fs::read_to_string(&path).map_err(|e| vec![format!("{}: 読めません: {e}", path.display())])?;
    parse(&text)
}

#[derive(Default)]
struct Draft {
    line: usize,
    name: String,
    desc: Option<String>,
    url: Option<String>,
}

/// 一覧の文を読む。誤りは `services.md: 3 行目: …` の形で、すべて返す。
pub fn parse(text: &str) -> Result<Content, Vec<String>> {
    let mut errors = Vec::new();
    let mut drafts: Vec<Draft> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let line = raw.trim();
        let mut err = |m: String| errors.push(format!("{FILE}: {n} 行目: {m}"));
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix("##") {
            let name = name.trim();
            if let Err(e) = check_name(name) {
                err(e);
            }
            if drafts.iter().any(|d| d.name.to_lowercase() == name.to_lowercase()) {
                err(format!("サービス「{name}」が 2 回あります。1 回にしてください"));
            }
            drafts.push(Draft { line: n, name: name.to_string(), ..Draft::default() });
            continue;
        }
        let Some(d) = drafts.last_mut() else {
            // 最初のサービスの前は、題（`# `）と覚え書き（`>`）だけ。公開しない
            if !(line.starts_with("# ") || line.starts_with('>')) {
                err("サービスは「## 名前」の行から書いてください（その前に書けるのは「# 」の題と「>」の覚え書きだけです）".into());
            }
            continue;
        };
        // 見出しの区切りは、最初の「:」か「：」（スマートフォンの入力で全角になっても読む。住所の「https:」で切らない）
        let Some((key, value)) = line.find([':', '：']).map(|i| {
            let sep = line[i..].chars().next().map_or(1, char::len_utf8);
            (&line[..i], &line[i + sep..])
        }) else {
            err("「説明: …」か「稼働状況: https://…」の形で書いてください".into());
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        let slot = match key {
            "説明" => &mut d.desc,
            "稼働状況" => &mut d.url,
            _ => {
                err(format!("知らない見出し「{key}」です。「説明」か「稼働状況」にしてください"));
                continue;
            }
        };
        if slot.is_some() {
            err(format!("「{key}」が 2 回あります。1 回にしてください"));
            continue;
        }
        let checked = if key == "説明" { check_desc(value) } else { check_url(value) };
        if let Err(e) = checked {
            err(e);
        }
        *slot = Some(value.to_string());
    }
    if drafts.is_empty() {
        errors.push(format!("{FILE}: サービスが 1 つもありません。「## 名前」の行から書いてください"));
    }
    let mut services = Vec::new();
    for d in drafts {
        match (d.desc, d.url) {
            (Some(desc), Some(url)) => services.push(Service { name: d.name, desc, url }),
            (desc, url) => {
                for (missing, key) in [(desc.is_none(), "説明"), (url.is_none(), "稼働状況")] {
                    if missing {
                        errors.push(format!("{FILE}: {} 行目: サービス「{}」に「{key}: …」の行がありません", d.line, d.name));
                    }
                }
            }
        }
    }
    check_personal(text, &mut errors);
    if errors.is_empty() { Ok(Content { services }) } else { Err(errors) }
}

fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("「## 」の後にサービスの名前を書いてください".into());
    }
    if name.chars().count() > MAX_NAME {
        return Err(format!("名前は {MAX_NAME} 字までにしてください"));
    }
    if name.chars().any(|c| c.is_control() || "<>&\"'`#".contains(c)) {
        return Err(format!("名前「{name}」に記号（< > & \" ' ` #）を使わないでください"));
    }
    Ok(())
}

fn check_desc(desc: &str) -> Result<(), String> {
    if desc.is_empty() {
        return Err("説明を書いてください（「……するサービスです。」の 1 文）".into());
    }
    if desc.chars().count() > MAX_DESC {
        return Err(format!("説明は {MAX_DESC} 字までにしてください（今は {} 字）", desc.chars().count()));
    }
    if !desc.ends_with('。') {
        return Err("説明は「。」で終えてください".into());
    }
    if desc.contains("http") || desc.contains('<') {
        return Err("説明にリンクやタグは書けません（文はそのまま表示します）".into());
    }
    for w in AVOID_WORDS {
        if desc.contains(w) {
            return Err(format!("説明に「{w}」と書かないでください（company の docs/design/guidelines.md 3.5）"));
        }
    }
    Ok(())
}

/// 稼働状況のページの住所。`https://` で始まり `/` で終わる。問い合わせ（`?`）や移動先（`#`）は付けない。
fn check_url(url: &str) -> Result<(), String> {
    let Some(rest) = url.strip_prefix("https://") else {
        return Err(format!("稼働状況のページの住所「{url}」は https:// から書いてください"));
    };
    if !url.ends_with('/') {
        return Err(format!("住所「{url}」は「/」で終えてください（例: https://status.nokorazu.jp/）"));
    }
    let host = rest.split('/').next().unwrap_or("");
    let host_ok = host.contains('.')
        && !host.starts_with(['.', '-'])
        && host.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-');
    if !host_ok {
        return Err(format!("住所「{url}」のドメインは、英小文字・数字・「.」・「-」で書いてください"));
    }
    if !rest.bytes().all(|b| b.is_ascii_alphanumeric() || b"./-_~".contains(&b)) {
        return Err(format!("住所「{url}」に使えない文字があります（? # 空白 引用符 など）"));
    }
    if url == crate::SITE_URL || url.starts_with(crate::REPO_URL) {
        return Err("このページ自身を指さないでください。サービスごとの稼働状況のページの住所を書いてください".into());
    }
    Ok(())
}

/// 個人情報らしいもの（メールアドレス・電話番号・郵便番号・〒）があれば誤りにする（公開のページとリポジトリなので）。
pub fn check_personal(text: &str, errors: &mut Vec<String>) {
    for addr in emails(text) {
        errors.push(format!("{FILE}: メールアドレス「{addr}」は書きません（公開のリポジトリです）"));
    }
    for num in phone_like(text) {
        errors.push(format!("{FILE}: 電話番号・郵便番号のような数字「{num}」は書きません（公開のリポジトリです）"));
    }
    if text.contains('〒') {
        errors.push(format!("{FILE}: 住所（〒）は書きません（公開のリポジトリです）"));
    }
}

/// 全角の数字・記号を半角にそろえる（電話番号の見つけ方に使う）。
pub fn normalize(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '０'..='９' => char::from_u32(c as u32 - '０' as u32 + '0' as u32).unwrap_or(c),
            '：' => ':',
            '　' => ' ',
            '－' | '−' | '‐' | 'ー' => '-',
            '（' => '(',
            '）' => ')',
            '＋' => '+',
            _ => c,
        })
        .collect()
}

/// 文の中のメールアドレス（`名前@ドメイン` の形。全角の ＠ も見る）。osakidev/nokorazu-status の source.rs から写した。
pub fn emails(text: &str) -> Vec<String> {
    let is_local = |c: char| c.is_ascii_alphanumeric() || "._%+-".contains(c);
    let is_domain = |c: char| c.is_ascii_alphanumeric() || ".-".contains(c);
    let mut out = Vec::new();
    for (i, _) in text.match_indices('@').chain(text.match_indices('＠')) {
        let local: String =
            text[..i].chars().rev().take_while(|c| is_local(*c)).collect::<Vec<_>>().into_iter().rev().collect();
        let rest = &text[i + if text[i..].starts_with('@') { 1 } else { '＠'.len_utf8() }..];
        let domain: String = rest.chars().take_while(|c| is_domain(*c)).collect();
        let domain = domain.trim_end_matches('.');
        // ドメインに「.」があるものだけ（CSS の @media などは除く）
        if domain.contains('.') && !domain.starts_with('.') {
            out.push(format!("{local}@{domain}"));
        }
    }
    out
}

/// 電話番号（0 から始まる 10 桁以上、+81）や郵便番号（3 桁-4 桁）に見える数字の並び。全角の数字も見る。
/// osakidev/nokorazu-status の source.rs から写した。
pub fn phone_like(text: &str) -> Vec<String> {
    let norm = normalize(text);
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in norm.chars().chain(['\n']) {
        if c.is_ascii_digit() || "-()+ ".contains(c) {
            cur.push(c);
            continue;
        }
        let s = cur.trim().to_string();
        cur.clear();
        let digits: String = s.chars().filter(char::is_ascii_digit).collect();
        let postal = s.len() == 8 && s.as_bytes().get(3) == Some(&b'-') && digits.len() == 7;
        if (digits.len() >= 10 && digits.starts_with('0')) || s.contains("+81") || postal {
            out.push(s);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 住所の文字() {
        let s = Service { name: "x".into(), desc: "x。".into(), url: "https://status.example.com/".into() };
        assert_eq!(s.address(), "status.example.com");
        let s = Service { url: "https://example.github.io/x-status/".into(), ..s };
        assert_eq!(s.address(), "example.github.io/x-status");
    }

    #[test]
    fn 住所の決まり() {
        for ok in ["https://status.nokorazu.jp/", "https://status.example.com/", "https://example.github.io/x-status/"] {
            assert!(check_url(ok).is_ok(), "{ok}");
        }
        for bad in [
            "http://status.example.com/",
            "https://status.example.com",
            "https://status.example.com/?x=1/",
            "https://status.example.com/#a/",
            "https://Status.example.com/",
            "https://localhost/",
            "https://status.example.com/a b/",
            "https://status.example.com/\"/",
            "javascript:alert(1)/",
            crate::SITE_URL,
        ] {
            assert!(check_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn 個人情報らしいもの() {
        assert_eq!(emails("連絡は user@example.com まで。"), ["user@example.com"]);
        assert_eq!(emails("ｘ＠example.com"), ["@example.com"]);
        assert!(emails("@media print { a { x: y } } @ 2026").is_empty());
        assert_eq!(phone_like("電話 090-0000-0000 まで"), ["090-0000-0000"]);
        assert_eq!(phone_like("０９０（００００）００００"), ["090(0000)0000"]);
        assert_eq!(phone_like("〒100-0000"), ["100-0000"]);
        assert!(phone_like("2026年11月23日、1.2.3 版、JIS X 8341-3:2016、20261026.354.1").is_empty());
    }
}
