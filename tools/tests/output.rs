//! 組み立てたページの確かめ。ガイドライン（osakidev/company docs/design/guidelines.md 版 1.1）のうち、機械で確かめられるもの。
//! osakidev/nokorazu-status の tests/output.rs を小さく写した。本物の一覧と、例の一覧（サービス 2 つ）から組み立てたページを見る。
//!
//! - ページの基本: lang・title・charset・viewport・referrer・見出しの順・ランドマーク・本文へ移動・スクリプトや style 属性を使わない など
//! - CSP: `<meta>` が先頭にあり、外部の出どころがない
//! - リンクとファイル: 中のリンクは相対（404.html だけ公開の場所のパスから）で、行き先がある。使われていないファイルがない
//! - 外部の URL: リンクは一覧のサービスと許可したもの（GitHub の説明・このリポジトリ）だけ。読み込みは同じ配信元だけ
//! - 文: 一覧のサービスがすべて出る。状態を書かない。プライバシーの説明
//! - 大きさ・書かない言葉・載せない情報・コントラスト・CSS の決まり
//! - コミット済みの docs/ が、一覧から組み立てたものと同じ

use std::collections::{BTreeMap, BTreeSet};

use status_tools::contrast::{floor2, parse_hex, ratio};
use status_tools::css;
use status_tools::html::{self, Tag, Token};
use status_tools::render;
use status_tools::site;
use status_tools::source::{self, AVOID_WORDS, Content};

/// 例の一覧（架空のサービス 2 つ。説明に「&」を入れて、文字のまま出ることも見る）。
const EXAMPLE: &str = "## 例のサービス\n説明: 例のためのサービスです（A&B）。\n稼働状況: https://status.example.com/\n\n## 二つ目の例\n説明: 二つ目の例のサービスです。\n稼働状況: https://example.github.io/two-status/\n";

/// 外に出てよいリンク（`<a href>` だけ）。ほかに一覧のサービスの稼働状況のページ。読み込み（`<link>`）はすべて同じ配信元に限る。
const ALLOWED_LINKS: [&str; 3] = [render::GITHUB_PAGES_DOC, render::GITHUB_PRIVACY, status_tools::REPO_URL];
/// 大きさの上限（遅い回線でも早く開けるように）。
const MAX_HTML: usize = 12 * 1024;
const MAX_CSS: usize = 8 * 1024;

struct Set {
    name: &'static str,
    content: Content,
    files: BTreeMap<String, Vec<u8>>,
}

fn sets() -> Vec<Set> {
    let real = source::load(&status_tools::repo_root()).unwrap_or_else(|e| panic!("{e:#?}"));
    let example = source::parse(EXAMPLE).unwrap_or_else(|e| panic!("{e:#?}"));
    [("本物（services.md）", real), ("例（サービス 2 つ）", example)]
        .into_iter()
        .map(|(name, content)| Set { name, files: site::build(&content), content })
        .collect()
}

struct Page {
    rel: String,
    tokens: Vec<Token>,
}

impl Page {
    fn open_tags(&self) -> impl Iterator<Item = &Tag> {
        html::tags(&self.tokens).filter(|t| !t.end)
    }
    fn ids(&self) -> BTreeSet<&str> {
        self.open_tags().filter_map(|t| t.attr("id")).collect()
    }
    fn readable(&self) -> String {
        let mut s = html::text(&self.tokens);
        for t in self.open_tags() {
            for a in ["title", "alt", "aria-label", "content"] {
                if let Some(v) = t.attr(a) {
                    s.push('\n');
                    s.push_str(v);
                }
            }
        }
        s
    }
}

fn pages(set: &Set) -> Vec<Page> {
    set.files
        .iter()
        .filter(|(k, _)| k.ends_with(".html"))
        .map(|(rel, body)| {
            let src = String::from_utf8(body.clone()).unwrap();
            Page { rel: rel.clone(), tokens: html::tokenize(&src).unwrap_or_else(|e| panic!("{}: {rel}: {e}", set.name)) }
        })
        .collect()
}

fn page(set: &Set, rel: &str) -> Page {
    pages(set).into_iter().find(|p| p.rel == rel).unwrap()
}

fn scheme(value: &str) -> Option<String> {
    let colon = value.find(':')?;
    let s = &value[..colon];
    let first_sep = value.find(['/', '?', '#']).unwrap_or(value.len());
    (colon < first_sep && !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c)))
        .then(|| s.to_ascii_lowercase())
}

const URL_ATTRS: [&str; 9] = ["href", "src", "srcset", "action", "formaction", "data", "poster", "cite", "xlink:href"];

#[test]
fn スキームの見分け方() {
    assert_eq!(scheme("https://status.nokorazu.jp/").as_deref(), Some("https"));
    assert_eq!(scheme("JavaScript:alert(1)").as_deref(), Some("javascript"));
    assert_eq!(scheme("style.css"), None);
    assert_eq!(scheme("../a:b"), None);
}

#[test]
fn ページの基本() {
    let mut errors = Vec::new();
    for set in sets() {
        let mut titles: BTreeMap<String, String> = BTreeMap::new();
        for page in pages(&set) {
            let p = format!("{}: {}", set.name, page.rel);
            let mut err = |m: String| errors.push(format!("{p}: {m}"));
            if let Err(e) = html::check_nesting(&page.tokens) {
                err(e);
            }
            let open: Vec<&Tag> = page.open_tags().collect();
            let count = |name: &str| open.iter().filter(|t| t.name == name).count();
            match open.first() {
                Some(t) if t.name == "html" && t.attr("lang") == Some("ja") => {}
                _ => err("最初の要素は <html lang=\"ja\">".into()),
            }
            let head: Vec<&&Tag> = open.iter().skip_while(|t| t.name != "head").skip(1).take(1).collect();
            if !matches!(head.first(), Some(t) if t.name == "meta" && t.attr("charset") == Some("utf-8")) {
                err("head の最初は <meta charset=\"utf-8\">".into());
            }
            let meta = |name: &str| open.iter().find(|t| t.name == "meta" && t.attr("name") == Some(name)).and_then(|t| t.attr("content"));
            if meta("viewport") != Some("width=device-width, initial-scale=1") {
                err("viewport は width=device-width, initial-scale=1（拡大を止めない）".into());
            }
            if meta("referrer") != Some("no-referrer") {
                err("<meta name=\"referrer\" content=\"no-referrer\">".into());
            }
            if meta("description").is_none_or(str::is_empty) {
                err("description がありません".into());
            }
            let mut inside = false;
            let mut title = String::new();
            for t in &page.tokens {
                match t {
                    Token::Tag(tag) if tag.name == "title" => inside = !tag.end,
                    Token::Text(x) if inside => title.push_str(x),
                    _ => {}
                }
            }
            let title = title.trim().to_string();
            if title.is_empty() {
                err("title がありません".into());
            } else if let Some(other) = titles.insert(title.clone(), page.rel.clone()) {
                err(format!("title「{title}」が {other} と同じ"));
            }
            for (name, n) in [("h1", 1), ("main", 1), ("header", 1), ("footer", 1)] {
                if count(name) != n {
                    err(format!("<{name}> は {n} つ（{} つある）", count(name)));
                }
            }
            if !open.iter().any(|t| t.name == "main" && t.attr("id") == Some("main")) {
                err("<main id=\"main\"> がありません".into());
            }
            match open.iter().find(|t| t.name == "a") {
                Some(t) if t.attr("href") == Some("#main") && t.attr("class") == Some("skip") => {}
                _ => err("最初のリンクは「本文へ移動」（<a class=\"skip\" href=\"#main\">）".into()),
            }
            let mut prev = 0;
            for t in &open {
                if let Some(level) = t.name.strip_prefix('h').and_then(|n| n.parse::<usize>().ok()).filter(|l| (1..=6).contains(l)) {
                    if level > prev + 1 {
                        err(format!("{} 行目: 見出しが h{prev} から h{level} に飛んでいます", t.line));
                    }
                    prev = level;
                }
            }
            for t in &open {
                let at = format!("{} 行目 <{}>", t.line, t.name);
                // スクリプト・埋め込み・フォーム・画像を使わない（画像もファビコンもないので CSP に img-src を書いていない）
                if ["script", "noscript", "style", "iframe", "embed", "object", "form", "input", "img", "picture", "svg", "video", "audio", "canvas", "base"].contains(&t.name.as_str()) {
                    err(format!("{at} は使わない"));
                }
                for (a, v) in &t.attrs {
                    if a == "style" {
                        err(format!("{at}: style 属性は使わない（CSP の style-src 'self'）"));
                    }
                    if a.starts_with("on") {
                        err(format!("{at}: {a} は使わない（スクリプトなし）"));
                    }
                    if a == "target" {
                        err(format!("{at}: 新しいタブで開かない"));
                    }
                    if a == "tabindex" && v != "0" && v != "-1" {
                        err(format!("{at}: tabindex は 0 か -1 だけ"));
                    }
                }
                if t.name == "meta" && t.attr("http-equiv").is_some_and(|v| v.eq_ignore_ascii_case("refresh")) {
                    err(format!("{at}: 自動で読み込み直さない（WCAG 2.2.1）"));
                }
                if t.name == "link" && t.attr("rel") != Some("stylesheet") {
                    err(format!("{at}: 読み込むのは style.css だけ（rel={:?}）", t.attr("rel")));
                }
            }
            let mut ids = BTreeSet::new();
            for t in &open {
                if let Some(id) = t.attr("id")
                    && !ids.insert(id)
                {
                    err(format!("id=\"{id}\" が 2 つあります"));
                }
            }
        }
    }
    assert!(errors.is_empty(), "{errors:#?}");
}

/// CSP の値に、外部の出どころや、守りを弱める指定がない。
fn csp_problems(csp: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut names = BTreeSet::new();
    for directive in csp.split(';').map(str::trim).filter(|d| !d.is_empty()) {
        let mut words = directive.split_whitespace();
        let name = words.next().unwrap_or("");
        names.insert(name.to_string());
        for src in words {
            if src != "'self'" && src != "'none'" {
                out.push(format!("{name} の {src}（'self' か 'none' だけにする）"));
            }
        }
    }
    for need in ["default-src", "style-src", "base-uri", "form-action"] {
        if !names.contains(need) {
            out.push(format!("{need} がありません"));
        }
    }
    if !csp.starts_with("default-src 'none'") {
        out.push("最初は default-src 'none'（許可したもの以外は読み込まない）".into());
    }
    if names.contains("frame-ancestors") {
        out.push("frame-ancestors は <meta> では効かないので書かない（効くように見せない）".into());
    }
    out
}

#[test]
fn csp_の確かめ方() {
    assert!(csp_problems(render::CSP).is_empty());
    for bad in [
        "default-src 'none'; style-src https://example.com; base-uri 'none'; form-action 'none'",
        "default-src 'none'; style-src 'self' 'unsafe-inline'; base-uri 'none'; form-action 'none'",
        "default-src 'none'; style-src 'self'; img-src *; base-uri 'none'; form-action 'none'",
        "default-src 'none'; style-src 'self'; img-src data:; base-uri 'none'; form-action 'none'",
        "default-src 'self'; style-src 'self'; base-uri 'none'; form-action 'none'",
        "default-src 'none'; style-src 'self'; base-uri 'none'",
        "default-src 'none'; style-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
    ] {
        assert!(!csp_problems(bad).is_empty(), "{bad}");
    }
}

#[test]
fn csp_は先頭の_meta_で外部の出どころがない() {
    let mut errors = Vec::new();
    for set in sets() {
        for page in pages(&set) {
            let open: Vec<&Tag> = page.open_tags().collect();
            let p = format!("{}: {}", set.name, page.rel);
            let metas: Vec<&&Tag> = open.iter().filter(|t| t.attr("http-equiv").is_some_and(|v| v.eq_ignore_ascii_case("Content-Security-Policy"))).collect();
            if metas.len() != 1 {
                errors.push(format!("{p}: CSP の meta が {} 個", metas.len()));
                continue;
            }
            let content = metas[0].attr("content").unwrap_or("");
            if content != render::CSP {
                errors.push(format!("{p}: CSP が決まりと違います: {content}"));
            }
            errors.extend(csp_problems(content).into_iter().map(|e| format!("{p}: {e}")));
            // charset のすぐ次に置く（読み込むものより前にないと効かない）
            let head: Vec<&&Tag> = open.iter().skip_while(|t| t.name != "head").skip(1).take(2).collect();
            if !matches!(head.get(1), Some(t) if t.attr("http-equiv").is_some()) {
                errors.push(format!("{p}: charset の次に CSP の meta を置く（GitHub Pages はヘッダーを付けられないため）"));
            }
        }
    }
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn 中のリンクとファイルがある() {
    let base = status_tools::base_path();
    let mut errors = Vec::new();
    for set in sets() {
        let pages = pages(&set);
        let mut referenced: BTreeSet<String> = BTreeSet::new();
        for page in &pages {
            for t in page.open_tags() {
                for (attr, value) in &t.attrs {
                    if !URL_ATTRS.contains(&attr.as_str()) {
                        continue;
                    }
                    let at = format!("{}: {}:{} <{} {attr}=\"{value}\">", set.name, page.rel, t.line, t.name);
                    if value.starts_with("//") || scheme(value).is_some() {
                        continue; // 外部（次のテスト）
                    }
                    let (path, frag) = match value.split_once('#') {
                        Some((p, f)) => (p, Some(f)),
                        None => (value.as_str(), None),
                    };
                    if path.contains('?') || path.contains("..") {
                        errors.push(format!("中のリンクに ? や .. を付けない: {at}"));
                        continue;
                    }
                    let file = if path.is_empty() {
                        page.rel.clone()
                    } else if page.rel == "404.html" {
                        // 404.html はどの URL でも出るので、公開の場所のパスから書く
                        match path.strip_prefix(base) {
                            Some("") => "index.html".to_string(),
                            Some(p) => p.to_string(),
                            None => {
                                errors.push(format!("404.html の中のリンクは {base} から書く: {at}"));
                                continue;
                            }
                        }
                    } else if path.starts_with('/') {
                        errors.push(format!("中のリンクは相対で書く（独自ドメインを付ける前後で同じに開けるように）: {at}"));
                        continue;
                    } else if path == "./" {
                        "index.html".to_string()
                    } else {
                        path.to_string()
                    };
                    if !set.files.contains_key(&file) {
                        errors.push(format!("ファイルがありません（{file}）: {at}"));
                        continue;
                    }
                    referenced.insert(file.clone());
                    if let Some(frag) = frag
                        && !page.ids().contains(frag)
                    {
                        errors.push(format!("移動先の id=\"{frag}\" がありません: {at}"));
                    }
                }
                if let Some(v) = t.attr("aria-labelledby").or(t.attr("aria-describedby")) {
                    for id in v.split_whitespace() {
                        if !page.ids().contains(id) {
                            errors.push(format!("{}: {}:{} aria の参照先 {id} がありません", set.name, page.rel, t.line));
                        }
                    }
                }
            }
        }
        for f in set.files.keys() {
            if !["index.html", "404.html", "CNAME", ".nojekyll"].contains(&f.as_str()) && !referenced.contains(f) {
                errors.push(format!("{}: どこからも使われていないファイル: {f}", set.name));
            }
        }
    }
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn 外部の_url_は一覧と許可したものだけ() {
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    for set in sets() {
        let services: BTreeSet<&str> = set.content.services.iter().map(|s| s.url.as_str()).collect();
        for page in pages(&set) {
            let mut linked = BTreeSet::new();
            for t in page.open_tags() {
                for (attr, value) in &t.attrs {
                    if !URL_ATTRS.contains(&attr.as_str()) {
                        continue;
                    }
                    let at = format!("{}: {}:{} <{} {attr}=\"{value}\">", set.name, page.rel, t.line, t.name);
                    if value.starts_with("//") {
                        errors.push(format!("スキームを省いた外部の URL: {at}"));
                        continue;
                    }
                    let Some(s) = scheme(value) else { continue };
                    let is_link = t.name == "a" && attr == "href";
                    if is_link && s == "https" && services.contains(value.as_str()) {
                        linked.insert(value.as_str());
                    } else if is_link && s == "https" && ALLOWED_LINKS.contains(&value.as_str()) {
                        seen.insert(value.clone());
                    } else {
                        errors.push(format!("許可していない外部の URL（読み込みは同じ配信元だけ。リンクは一覧と許可した先だけ）: {at}"));
                    }
                }
            }
            // 一覧のページと 404.html の両方に、すべてのサービスへのリンクがある
            if linked != services {
                errors.push(format!("{}: {}: サービスへのリンクが {linked:?}（{services:?} にする）", set.name, page.rel));
            }
        }
    }
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(seen.len(), ALLOWED_LINKS.len(), "使っていない許可は消す: {seen:?}");
}

#[test]
fn 一覧のサービスがすべて出る() {
    for set in sets() {
        let index = page(&set, "index.html");
        let text = index.readable();
        let open: Vec<&Tag> = index.open_tags().collect();
        let items = open.iter().skip_while(|t| t.attr("class") != Some("services")).filter(|t| t.name == "h3").count();
        assert_eq!(items, set.content.services.len(), "{}: サービスごとに見出し（h3）が 1 つ", set.name);
        assert!(open.iter().any(|t| t.name == "ul" && t.attr("class") == Some("services") && t.attr("role") == Some("list")), "{}", set.name);
        for s in &set.content.services {
            for want in [s.name.clone(), s.desc.clone(), format!("{} の稼働状況（{}）", s.name, s.address())] {
                assert!(text.contains(&want), "{}: 「{want}」がない", set.name);
            }
        }
        // 並びは一覧の並び
        let names: Vec<usize> = set.content.services.iter().map(|s| text.find(&format!("{} の稼働状況（", s.name)).unwrap()).collect();
        assert!(names.windows(2).all(|w| w[0] < w[1]), "{}: 並びが一覧と違う", set.name);
    }
    // 記号は文字のまま出る（タグにならない）
    let example = sets().remove(1);
    assert!(String::from_utf8(example.files["index.html"].clone()).unwrap().contains("（A&amp;B）"));
}

/// このページはサービスの状態を持たない（二重に書かない）。状態の言葉や、状態を示す部品を出さない。
#[test]
fn サービスの状態を書かない() {
    for set in sets() {
        for page in pages(&set) {
            let text = page.readable();
            for bad in ["正常", "停止", "一部に問題", "保守中", "保守の予定", "公開前", "障害が起き", "復旧"] {
                assert!(!text.contains(bad), "{}: {}: 「{bad}」と書かない（状態はサービスごとのページが持つ）", set.name, page.rel);
            }
        }
    }
    let text = page(&sets().remove(0), "index.html").readable();
    assert!(text.contains("お使いのサービスのページを開いてください。"));
    assert!(text.contains("障害や保守のお知らせは、それぞれのページに載せています。このページには載せていません。"));
}

#[test]
fn 読み上げの順() {
    let index = page(&sets().remove(1), "index.html");
    let mut heads = Vec::new();
    let mut inside = false;
    let mut cur = String::new();
    for t in &index.tokens {
        match t {
            Token::Tag(tag) if matches!(tag.name.as_str(), "h1" | "h2" | "h3") => {
                if tag.end {
                    heads.push(format!("{} {}", tag.name, cur.trim()));
                } else {
                    cur.clear();
                }
                inside = !tag.end;
            }
            Token::Text(x) if inside => cur.push_str(x),
            _ => {}
        }
    }
    assert_eq!(heads, ["h1 OSAKI Dev の稼働状況", "h2 サービスの一覧", "h3 例のサービス", "h3 二つ目の例", "h2 このページについて"]);
    for rule in &style().rules {
        for d in &rule.decls {
            let at = format!("{} {{ {}: {} }}", rule.selector, d.prop, d.value);
            assert!(d.prop != "order" && d.prop != "grid-template-areas" && d.prop != "float", "CSS で順番を変えない: {at}");
            assert!(!d.value.contains("reverse"), "{at}");
        }
    }
}

#[test]
fn プライバシーの説明と運営者() {
    for set in sets() {
        let text = page(&set, "index.html").readable();
        // nokorazu の稼働状況のページの「このページについて」と同じ言い方
        for want in [
            "GitHub Pages から配信しています",
            "Cookie・スクリプト・アクセス解析・外部からの読み込みを使いません",
            "IP アドレスを、セキュリティのために記録して保存します",
            "運営者は受け取りません",
            "利用者の地域や米国など",
            "osakidev/status",
        ] {
            assert!(text.contains(want), "{}: 「{want}」がない", set.name);
        }
        for page in pages(&set) {
            assert!(page.readable().contains("運営: OSAKI Dev（大崎開発）"), "{}: {}", set.name, page.rel);
        }
    }
}

#[test]
fn 見つからないときのページ() {
    let set = sets().remove(0);
    let p = page(&set, "404.html");
    let base = status_tools::base_path();
    let hrefs: Vec<&str> = p.open_tags().filter_map(|t| t.attr("href")).collect();
    assert!(hrefs.contains(&format!("{base}style.css").as_str()) && hrefs.iter().filter(|h| **h == base).count() == 2, "{hrefs:?}");
    assert!(p.readable().contains("OSAKI Dev の稼働状況の一覧を見る"));
}

#[test]
fn 小さく保つ() {
    for set in sets() {
        for (rel, body) in &set.files {
            let max = if rel.ends_with(".html") { MAX_HTML } else if rel.ends_with(".css") { MAX_CSS } else { 1024 };
            assert!(body.len() <= max, "{}: {rel} が {} バイト（{max} まで）", set.name, body.len());
        }
    }
}

#[test]
fn 書かない言葉() {
    let mut errors = Vec::new();
    for set in sets() {
        for page in pages(&set) {
            let text = page.readable();
            for w in AVOID_WORDS {
                if text.contains(w) {
                    errors.push(format!("{}: {}: 「{w}」と書かない（ガイドライン 3.5・13.）", set.name, page.rel));
                }
            }
        }
    }
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn 住所_電話番号_メールアドレスを載せない() {
    let mut errors = Vec::new();
    for set in sets() {
        for (rel, body) in &set.files {
            let text = String::from_utf8_lossy(body);
            for addr in source::emails(&text) {
                errors.push(format!("{}: {rel}: メールアドレス {addr}", set.name));
            }
            for num in source::phone_like(&text) {
                errors.push(format!("{}: {rel}: 電話番号・郵便番号のような数字 {num}", set.name));
            }
            if text.contains('〒') {
                errors.push(format!("{}: {rel}: 〒", set.name));
            }
        }
    }
    assert!(errors.is_empty(), "{errors:#?}");
}

// ---- CSS ----

fn style() -> css::Sheet {
    css::parse(site::STYLE).unwrap_or_else(|e| panic!("style.css: {e}"))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Role {
    Background,
    Text,
    NonText,
    Decorative,
}

/// 色の変数の役割。変数を足したら、ここにも足す（足さないとテストが落ちる）。
const ROLES: [(&str, Role); 7] = [
    ("--bg", Role::Background),
    ("--panel", Role::Background),
    ("--fg", Role::Text),
    ("--muted", Role::Text),
    ("--accent", Role::Text),
    ("--focus", Role::NonText),
    ("--line", Role::Decorative),
];

#[test]
fn コントラスト() {
    let themes = css::color_tokens(&style()).unwrap();
    let names: BTreeSet<&str> = themes.light.keys().map(String::as_str).collect();
    let roles: BTreeSet<&str> = ROLES.iter().map(|r| r.0).collect();
    assert_eq!(names, roles, "色の変数と、このテストの ROLES が食い違っています");
    for (label, only) in [("ダーク", &themes.dark_only), ("印刷", &themes.print_only)] {
        let keys: BTreeSet<&str> = only.keys().map(String::as_str).collect();
        assert_eq!(keys, names, "{label}の :root で置き直していない色の変数があります");
    }
    let mut failures = Vec::new();
    println!("| テーマ | 前景 | 背景 | 値 | 比（切り捨て） | 必要 |");
    println!("|---|---|---|---|---|---|");
    for (theme, values) in [("ライト", themes.light.clone()), ("ダーク", themes.dark()), ("印刷", themes.print())] {
        let color = |name: &str| parse_hex(&values[name]).unwrap();
        for &(fg, role) in &ROLES {
            let need = match role {
                Role::Background => continue,
                Role::Text => Some(4.5),
                Role::NonText => Some(3.0),
                Role::Decorative => None,
            };
            for &(bg, bg_role) in &ROLES {
                if bg_role != Role::Background {
                    continue;
                }
                let r = ratio(color(fg), color(bg));
                println!("| {theme} | {fg} | {bg} | {} / {} | {:.2} | {} |", values[fg], values[bg], floor2(r), need.map_or("参考".into(), |n| format!("{n}:1")));
                if need.is_some_and(|n| r < n) {
                    failures.push(format!("{theme}: {fg} / {bg} = {:.2}", floor2(r)));
                }
            }
        }
    }
    assert!(failures.is_empty(), "比が足りません: {failures:?}");
    // 会社の色（浅葱。ガイドライン 4.1）
    assert_eq!(themes.light["--accent"], "#0e7c7b");
    assert_eq!(themes.dark()["--accent"], "#5cc8c2");
    // theme-color はページの背景と同じ
    let index = String::from_utf8(sets().remove(0).files["index.html"].clone()).unwrap();
    assert!(index.contains(&format!("content=\"{}\" media=\"(prefers-color-scheme: light)\"", themes.light["--bg"])));
    assert!(index.contains(&format!("content=\"{}\" media=\"(prefers-color-scheme: dark)\"", themes.dark()["--bg"])));
}

#[test]
fn css_の決まり() {
    let sheet = style();
    let text = site::STYLE;
    let mut errors = Vec::new();
    assert!(sheet.statements.is_empty(), "@import などは使わない: {:?}", sheet.statements);
    let lower = text.to_ascii_lowercase();
    for bad in ["url(", "@font-face", "@import"] {
        if lower.contains(bad) {
            errors.push(format!("外部のファイルを読み込む書き方 {bad} を使わない"));
        }
    }
    let mut defined: BTreeSet<String> = BTreeSet::new();
    for rule in sheet.rules.iter().filter(|r| r.selector == ":root") {
        defined.extend(rule.decls.iter().filter(|d| d.prop.starts_with("--")).map(|d| d.prop.clone()));
    }
    let mut used: BTreeSet<String> = BTreeSet::new();
    for rule in &sheet.rules {
        let forced = rule.at.iter().any(|a| a.contains("forced-colors: active"));
        for d in &rule.decls {
            let at = format!("{} {{ {}: {} }}", rule.selector, d.prop, d.value);
            let mut rest = d.value.as_str();
            while let Some(i) = rest.find("var(") {
                let name: String = rest[i + 4..].chars().take_while(|c| *c != ')' && *c != ',').collect();
                if !defined.contains(name.trim()) {
                    errors.push(format!("定義していない変数 {name}: {at}"));
                }
                used.insert(name.trim().to_string());
                rest = &rest[i + 4..];
            }
            let token_def = rule.selector == ":root" && d.prop.starts_with("--");
            if !token_def {
                let lits = css::color_literals(&d.value, forced);
                if !lits.is_empty() {
                    errors.push(format!("色を直接書いています {lits:?}: {at}"));
                }
            }
            match d.prop.as_str() {
                "font" => errors.push(format!("font の一括指定は使わない: {at}")),
                "font-size" => {
                    if let Err(e) = check_font_size(&d.value) {
                        errors.push(format!("{e}: {at}"));
                    }
                }
                "outline" | "outline-style" if matches!(d.value.as_str(), "none" | "0") => {
                    errors.push(format!("フォーカスの輪郭線を消さない（ガイドライン 7.）: {at}"));
                }
                "position" if matches!(d.value.as_str(), "fixed" | "sticky") => {
                    errors.push(format!("固定表示を使わない（ガイドライン 6.）: {at}"));
                }
                _ => {}
            }
            // 余白は 7 段だけ（ガイドライン 6.）
            if ["margin", "padding", "gap"].iter().any(|p| d.prop.starts_with(p)) {
                for v in d.value.split_whitespace() {
                    if v.ends_with("rem") && !["0.25rem", "0.5rem", "0.75rem", "1rem", "1.5rem", "2rem", "3rem"].contains(&v) {
                        errors.push(format!("余白は 0.25 / 0.5 / 0.75 / 1 / 1.5 / 2 / 3rem だけ: {at}"));
                    }
                }
            }
        }
    }
    for name in &defined {
        if !used.contains(name) {
            errors.push(format!("使っていない変数 {name}"));
        }
    }
    // 印刷と強制カラーの指定がある（ガイドライン 9.）
    assert!(sheet.rules.iter().any(|r| r.at.iter().any(|a| a == "@media print") && r.selector.contains(".skip")), "印刷では本文へ移動を隠す");
    assert!(sheet.rules.iter().any(|r| r.at.iter().any(|a| a.contains("forced-colors: active"))), "強制カラーの指定");
    assert!(errors.is_empty(), "{errors:#?}");
}

fn check_font_size(value: &str) -> Result<(), String> {
    let v = value.trim();
    let split = v.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(v.len());
    let (num, unit) = v.split_at(split);
    let x: f64 = num.parse().map_err(|_| format!("数が読めません {v}"))?;
    match unit {
        "rem" if x >= 0.875 => Ok(()),
        "rem" => Err(format!("0.875rem より小さい {v}")),
        _ => Err(format!("文字の大きさは rem で書く: {v}")),
    }
}

// ---- コミット済みの出力 ----

/// docs/ が一覧から組み立てたものと同じ（Actions でも、配信の前に確かめる）。
#[test]
fn docs_が一覧と合っている() {
    let c = source::load(&status_tools::repo_root()).unwrap();
    let d = site::diff(&status_tools::docs_dir(), &site::build(&c));
    assert!(d.is_empty(), "docs/ が古いままです。./build.sh で組み立て直して、docs/ もコミットしてください:\n{}", d.join("\n"));
    // 独自ドメインを付けるまでは CNAME を置かない
    assert_eq!(status_tools::docs_dir().join("CNAME").exists(), status_tools::DOMAIN.is_some());
}

#[test]
fn 同じ一覧からはいつも同じ一式() {
    for set in sets() {
        assert_eq!(site::build(&set.content), set.files, "{}", set.name);
    }
}
