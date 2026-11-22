//! サービスの一覧（`services.md`）から、配信するファイル（`docs/`）を組み立てる。
//!
//! ```sh
//! ./build.sh                  # 組み立てて docs/ に書く
//! ./build.sh --check          # 書かずに、docs/ が一覧と合っているかだけ確かめる
//! ./build.sh --out 出力先      # 別の場所に組み立てる（Actions の配信）
//! ```
//!
//! 一覧に誤りがあれば、何も書かずに 1 で終わる（誤りはすべて、行の番号と直し方つきで出す）。
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use status_tools::{site, source};

fn main() -> ExitCode {
    let mut out: Option<PathBuf> = None;
    let mut check = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out = args.next().map(PathBuf::from),
            "--check" => check = true,
            _ => {
                eprintln!("使い方: render [--out 出力先] [--check]");
                return ExitCode::from(2);
            }
        }
    }
    let out = out.unwrap_or_else(status_tools::docs_dir);
    // Actions の中では、実行の一覧の画面にも出す
    let actions = std::env::var_os("GITHUB_ACTIONS").is_some();
    let c = match source::load(&status_tools::repo_root()) {
        Ok(c) => c,
        Err(errors) => {
            eprintln!("{} に直すところがあります（{} 件）。何も書いていません。", source::FILE, errors.len());
            for e in errors {
                if actions {
                    println!("::error title={} に直すところがあります::{e}", source::FILE);
                }
                eprintln!("- {e}");
            }
            return ExitCode::FAILURE;
        }
    };
    let files = site::build(&c);
    println!("サービス {} 件、配信するファイル {} 個", c.services.len(), files.len());
    if check {
        let d = site::diff(&out, &files);
        if d.is_empty() {
            println!("{} は {} と合っています。", out.display(), source::FILE);
            return ExitCode::SUCCESS;
        }
        eprintln!("{} が {} と合っていません。./build.sh で組み立て直して、docs/ もコミットしてください。", out.display(), source::FILE);
        for x in d {
            eprintln!("- {x}");
        }
        return ExitCode::FAILURE;
    }
    match site::write(&out, &files) {
        Ok(changed) if changed.is_empty() => {
            println!("{} は変わりませんでした。", out.display());
            ExitCode::SUCCESS
        }
        Ok(changed) => {
            println!("{} に書きました（変わったファイル {} 個）:", out.display(), changed.len());
            for c in changed {
                println!("- {c}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("書けませんでした: {e}");
            ExitCode::FAILURE
        }
    }
}
