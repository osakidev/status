//! HTML のページを作る。スクリプト・style 属性・画像・外部の読み込みは使わない（CSP は `<meta>` で付ける。GitHub Pages は
//! HTTP のヘッダーを足せないため）。
//!
//! - ページの中のリンクは相対にする（`style.css`）。独自ドメインがない今の `https://osakidev.github.io/status/` でも、
//!   独自ドメインを付けた後も、同じに開ける。見つからないときのページ（404.html）だけは、どの URL でも出るので
//!   公開の場所のパス（`crate::base_path()`）から書く
//! - このページはサービスの状態を書かない。サービスごとの稼働状況のページへ案内するだけ
//! - 読み上げの順は HTML の順。CSS で順番を変えない（tests/output.rs が確かめる）

use crate::source::{Content, Service};
use crate::{REPO_URL, base_path};

/// `<meta>` で付ける CSP。画像（ファビコンを含む）も使わないので `img-src` は書かない（`default-src 'none'` で止まる）。
/// `frame-ancestors` は `<meta>` では効かないので書かない（README の 6.1）。
pub const CSP: &str = "default-src 'none'; style-src 'self'; base-uri 'none'; form-action 'none'";

/// ページの名前（見出しと、ヘッダーの文字）。
pub const SITE_NAME: &str = "OSAKI Dev の稼働状況";
/// 運営者の表示（company の docs/company/00-overview.md。法人化の前なので「Inc.」「株式会社」を付けない）。
pub const OPERATOR: &str = "OSAKI Dev（大崎開発）";

/// GitHub Pages の説明（見に来た人の IP アドレスの記録。「データ コレクション」の節）。
pub const GITHUB_PAGES_DOC: &str = "https://docs.github.com/ja/pages/getting-started-with-github-pages/about-github-pages";
/// GitHub のプライバシーに関する説明。
pub const GITHUB_PRIVACY: &str = "https://docs.github.com/ja/site-policy/privacy-policies/github-general-privacy-statement";

/// 文字を HTML に入れられる形にする。
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

pub struct Page<'a> {
    pub title: &'a str,
    pub description: &'a str,
    /// ページから公開の場所の一番上への道（`""`。404.html だけ `base_path()`）
    pub root: &'a str,
    pub main: String,
}

/// ページの骨組み（head・本文へ移動・ヘッダー・フッター）。
pub fn page(p: &Page) -> String {
    let r = p.root;
    let full_title = if p.title == SITE_NAME { p.title.to_string() } else { format!("{}｜{SITE_NAME}", p.title) };
    format!(
        r##"<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="{csp}">
<meta name="referrer" content="no-referrer">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<meta name="description" content="{desc}">
<meta name="color-scheme" content="light dark">
<meta name="theme-color" content="#f7f6f2" media="(prefers-color-scheme: light)">
<meta name="theme-color" content="#1b1c1e" media="(prefers-color-scheme: dark)">
<link rel="stylesheet" href="{r}style.css">
</head>
<body>
<a class="skip" href="#main">本文へ移動</a>
<header class="site-header">
  <div class="wrap">
    <a class="brand" href="{home}">{SITE_NAME}</a>
  </div>
</header>

<main id="main">
  <div class="wrap">
{main}  </div>
</main>

<footer class="site-footer">
  <div class="wrap">
    <p class="operator">運営: {OPERATOR}</p>
  </div>
</footer>
</body>
</html>
"##,
        csp = CSP,
        title = esc(&full_title),
        desc = esc(p.description),
        home = if r.is_empty() { "./" } else { r },
        main = p.main,
    )
}

/// サービスの稼働状況のページへのリンク。文だけで行き先が分かるように、住所も添える。
pub fn status_link(s: &Service) -> String {
    format!("<a href=\"{}\">{} の稼働状況（{}）</a>", esc(&s.url), esc(&s.name), esc(s.address()))
}

/// 一覧のページ（index.html）。読み上げの順は、見出し → このページの使い方 → サービスの一覧 → このページについて。
pub fn index(c: &Content) -> String {
    let mut m = format!("<h1>{SITE_NAME}</h1>\n");
    m.push_str(&format!(
        "<p>{OPERATOR}のサービスごとに、稼働状況のページがあります。お使いのサービスのページを開いてください。</p>\n<p>サービスが今使えるかどうかと、障害や保守のお知らせは、それぞれのページに載せています。このページには載せていません。</p>\n"
    ));
    // list-style: none の一覧は、Safari の読み上げで一覧と伝わらないことがあるので role="list" を付ける
    m.push_str("<section aria-labelledby=\"services\">\n<h2 id=\"services\">サービスの一覧</h2>\n<ul class=\"services\" role=\"list\">\n");
    for s in &c.services {
        m.push_str(&format!("<li>\n<h3>{}</h3>\n<p>{}</p>\n<p>{}</p>\n</li>\n", esc(&s.name), esc(&s.desc), status_link(s)));
    }
    m.push_str("</ul>\n</section>\n");
    m.push_str(&about());
    page(&Page {
        title: SITE_NAME,
        description: "OSAKI Dev（大崎開発）のサービスごとの、稼働状況のページの一覧です。",
        root: "",
        main: m,
    })
}

/// このページについて。nokorazu の稼働状況のページの「このページについて」と同じ言い方にした。
fn about() -> String {
    let repo = REPO_URL.strip_prefix("https://github.com/").unwrap_or(REPO_URL);
    format!(
        r#"<section aria-labelledby="about">
<h2 id="about">このページについて</h2>
<ul>
<li>このページは、GitHub Pages から配信しています。</li>
<li>このページは、Cookie・スクリプト・アクセス解析・外部からの読み込みを使いません。</li>
<li>配信している GitHub は、見に来た方の IP アドレスを、セキュリティのために記録して保存します。詳しくは、<a href="{GITHUB_PAGES_DOC}">GitHub Pages についての GitHub の説明</a>をご覧ください。この記録は GitHub が自ら扱うもので、運営者は受け取りません。扱いは、<a href="{GITHUB_PRIVACY}">GitHub のプライバシーに関する説明</a>によります。GitHub は、利用者の地域や米国などで情報を扱うとしています。</li>
<li>このページの書き換えの記録は、GitHub の <a href="{REPO_URL}">{repo}</a> で、どなたでも見られます。</li>
</ul>
</section>
"#
    )
}

/// 見つからないとき（404.html）。GitHub Pages はどの URL にもこのファイルを返すので、リンクは公開の場所のパスから書く。
/// 古いリンクから来た方のために、サービスの稼働状況のページへのリンクも載せる。
pub fn not_found(c: &Content) -> String {
    let base = base_path();
    let mut m = String::from(
        "<h1>ページが見つかりません</h1>\n<p>お探しのページは、ここにはありません。住所（URL）をお確かめください。</p>\n<p>サービスの稼働状況は、次のページに載せています。</p>\n<ul>\n",
    );
    for s in &c.services {
        m.push_str(&format!("<li>{}</li>\n", status_link(s)));
    }
    m.push_str(&format!("</ul>\n<p><a href=\"{base}\">{SITE_NAME}の一覧を見る</a></p>\n"));
    page(&Page { title: "ページが見つかりません", description: "お探しのページは見つかりませんでした。", root: base, main: m })
}
