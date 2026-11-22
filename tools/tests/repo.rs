//! リポジトリ全体の決まりの確かめ（osakidev/nokorazu-status の tests/repo.rs を写した）。
//!
//! - 依存ゼロ（サードパーティのクレートを足さない）。組み立ては依存もネットワークも使わない
//! - ワークフロー: アクションは公式のものを SHA で固定、権限は最小、シークレットなし、配信は確かめの後
//! - 公開のリポジトリに個人情報を置かない

use std::path::{Path, PathBuf};

use status_tools::repo_root;
use status_tools::source::{emails, phone_like};

fn read(rel: &str) -> String {
    std::fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

#[test]
fn 依存がない() {
    let toml = read("tools/Cargo.toml");
    let deps = toml.split("[dependencies]").nth(1).expect("[dependencies] の節");
    assert!(deps.trim().is_empty(), "依存を足さない: {deps}");
    assert!(!toml.contains("[dev-dependencies]") && !toml.contains("[build-dependencies]"));
    let lock = read("tools/Cargo.lock");
    assert_eq!(lock.matches("[[package]]").count(), 1, "Cargo.lock のパッケージは自分だけ: {lock}");
}

#[test]
fn 組み立ては依存もネットワークも使わない() {
    for script in ["build.sh", "test.sh"] {
        let text = read(script);
        assert!(text.contains("--locked --offline"), "{script}");
        for bad in ["curl", "wget", "aws "] {
            assert!(!text.contains(bad), "{script}: {bad}");
        }
    }
}

/// ワークフローの問題を探す（company の docs/security/github.md の 5 章）。
fn workflow_problems(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    if !text.lines().any(|l| l == "permissions: {}") {
        out.push("最上位は permissions: {}（既定は権限なし）".to_string());
    }
    let code: Vec<&str> = text.lines().filter(|l| !l.trim_start().starts_with('#')).collect();
    let mut checkouts = 0;
    for line in &code {
        let Some((_, rest)) = line.split_once("uses: ") else { continue };
        let (action, comment) = rest.split_once(" # ").unwrap_or((rest, ""));
        let (name, sha) = action.split_once('@').unwrap_or((action, ""));
        if !name.starts_with("actions/") {
            out.push(format!("GitHub の公式のアクションだけ: {line}"));
        }
        if sha.len() != 40 || !sha.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            out.push(format!("コミットの SHA（40 桁）で固定する: {line}"));
        }
        if !comment.starts_with('v') {
            out.push(format!("行末に版のコメントを書く: {line}"));
        }
        if name == "actions/checkout" {
            checkouts += 1;
        }
    }
    let body = code.join("\n");
    if body.matches("persist-credentials: false").count() != checkouts {
        out.push("checkout には persist-credentials: false".to_string());
    }
    for bad in ["pull_request_target", "workflow_run", "secrets.", "github.event.", "curl ", "wget ", "aws-actions", "configure-aws", "amazonaws", "contents: write", "permissions: write-all", "continue-on-error"] {
        if body.contains(bad) {
            out.push(format!("使わない: {bad}"));
        }
    }
    // 書き込みの権限は配信のジョブだけ。配信は確かめ（test）の後（test → build → deploy）
    let (before, deploy) = body.split_once("\n  deploy:").unwrap_or((&body, ""));
    if before.contains(": write") {
        out.push("書き込みの権限は deploy のジョブだけ".to_string());
    }
    for need in ["pages: write", "id-token: write", "needs: build", "name: github-pages"] {
        if !deploy.contains(need) {
            out.push(format!("deploy のジョブに {need} がない"));
        }
    }
    let build = before.split_once("\n  build:").map_or("", |(_, b)| b);
    if !build.contains("needs: test") {
        out.push("build のジョブは needs: test（確かめが通ってから配信する）".to_string());
    }
    out
}

#[test]
fn ワークフローは固定して権限を最小にする() {
    let text = read(".github/workflows/pages.yml");
    let problems = workflow_problems(&text);
    assert!(problems.is_empty(), "{problems:#?}");
    assert!(text.contains("./build.sh --out _site"), "配信するのは一覧から組み立てたもの");
    assert!(text.contains("run: ./test.sh\n"), "確かめは docs/ が最新かも含めてすべて流す");
}

#[test]
fn ワークフローの問題の見つけ方() {
    let good = read(".github/workflows/pages.yml");
    let sha = "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1";
    assert!(good.contains(sha));
    for (bad, want) in [
        (good.replace(sha, "actions/checkout@v7"), "SHA"),
        (good.replace(sha, "someone/action@3d3c42e5aac5ba805825da76410c181273ba90b1 # v1"), "公式"),
        (good.replace(sha, "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1"), "版のコメント"),
        (good.replace("permissions: {}", "permissions: write-all"), "権限なし"),
        (good.replace("  pull_request:\n", "  pull_request_target:\n"), "pull_request_target"),
        (good.replace("persist-credentials: false", "persist-credentials: true"), "persist-credentials"),
        (good.replace("      contents: read", "      contents: write"), "contents: write"),
        (good.replace("needs: build", "needs: []"), "needs: build"),
        (good.replace("    needs: test\n", ""), "needs: test"),
    ] {
        let p = workflow_problems(&bad);
        assert!(p.iter().any(|x| x.contains(want)), "{want}: {p:?}");
    }
}

#[test]
fn dependabot_がアクションの版を見る() {
    let text = read(".github/dependabot.yml");
    assert!(text.contains("package-ecosystem: github-actions") && text.contains("interval: weekly"));
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        if name == ".git" || name == "target" || name == "_site" {
            continue;
        }
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

#[test]
fn 公開のリポジトリに個人情報を置かない() {
    let root = repo_root();
    let mut files = Vec::new();
    walk(&root, &mut files);
    assert!(files.len() > 15, "{}", files.len());
    // 例に使ってよいもの（RFC 2606 の例示用、README の push の手順の SSH の宛先）
    let allowed = ["@example.com", "@example.invalid", "git@github.com"];
    // 古いアカウント名・個人のメールのサービス（この文字列そのものをリポジトリに置かないように、分けて書く）
    let banned = [concat!("kosaki", "1983"), concat!("@", "gmail"), concat!("osaki.", "kengo@")];
    let mut errors = Vec::new();
    for path in &files {
        let rel = path.strip_prefix(&root).unwrap().to_string_lossy().into_owned();
        let Ok(text) = std::fs::read_to_string(path) else { continue };
        for addr in emails(&text) {
            if !allowed.iter().any(|d| addr.ends_with(d)) {
                errors.push(format!("{rel}: メールアドレス {addr}"));
            }
        }
        for bad in banned {
            if text.contains(bad) {
                errors.push(format!("{rel}: {bad}"));
            }
        }
        // 公開する文には、電話番号・郵便番号に見える数字を置かない（道具のコードとテストの架空の値は除く）
        if !rel.starts_with("tools/") {
            for num in phone_like(&text) {
                errors.push(format!("{rel}: 電話番号・郵便番号のような数字 {num}"));
            }
            if text.contains('〒') {
                errors.push(format!("{rel}: 〒"));
            }
        }
    }
    assert!(errors.is_empty(), "{errors:#?}");
}
