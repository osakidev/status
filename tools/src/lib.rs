//! OSAKI Dev の稼働状況のまとめのページの道具。標準ライブラリだけで書く（外部クレートなし）。
//!
//! - `source`: サービスの一覧（`services.md`）を読み、決まりどおりか確かめる
//! - `render`: HTML のページ（一覧と、見つからないとき）
//! - `site`: 配信するファイルの一式を作る・書き出す・コミット済みのもの（`docs/`）と比べる
//! - `html`・`css`・`contrast`: 確かめ（テスト）のための小さな解析と、コントラスト比
//!
//! このページは各サービスの状態を持たない。状態はサービスごとの稼働状況のページ（`<サービス>-status`）が持つ。
#![forbid(unsafe_code)]

pub mod contrast;
pub mod css;
pub mod html;
pub mod render;
pub mod site;
pub mod source;

use std::path::PathBuf;

/// 公開の場所。独自ドメイン（`status.osaki.dev`）を付けるまでは、GitHub Pages の既定の場所。
/// 付けるときは、ここを `https://status.osaki.dev/` に、`DOMAIN` を `Some("status.osaki.dev")` にする（README の 5.2）。
pub const SITE_URL: &str = "https://osakidev.github.io/status/";

/// 独自ドメイン（`docs/CNAME`）。付けるまでは `None` で、`docs/CNAME` を作らない。
pub const DOMAIN: Option<&str> = None;

/// 公開のリポジトリ。書き換えの記録を、どなたでも見られる場所。名前は `status`（company の `docs/company/12-naming.md`）。
pub const REPO_URL: &str = "https://github.com/osakidev/status";

/// 公開の場所の、ドメインの後ろのパス（`/status/` か `/`）。見つからないときのページ（404.html）は、どの URL でも
/// 出るので、リンクをここから書く。
pub fn base_path() -> &'static str {
    let rest = SITE_URL.strip_prefix("https://").unwrap_or(SITE_URL);
    rest.find('/').map_or("/", |i| &rest[i..])
}

/// リポジトリ直下（このクレートの 1 つ上）。
pub fn repo_root() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    p.canonicalize().unwrap_or(p)
}

/// 配信するディレクトリ（GitHub Pages の決まりで、ブランチから配信するときに選べる名前は `/` か `/docs` だけ）。
pub fn docs_dir() -> PathBuf {
    repo_root().join("docs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 公開の場所と独自ドメインが合っている() {
        assert!(SITE_URL.starts_with("https://") && SITE_URL.ends_with('/'));
        match DOMAIN {
            Some(d) => {
                assert_eq!(SITE_URL, format!("https://{d}/"));
                assert_eq!(base_path(), "/");
            }
            None => {
                assert_eq!(SITE_URL, "https://osakidev.github.io/status/");
                assert_eq!(base_path(), "/status/");
            }
        }
    }
}
