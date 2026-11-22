//! 配信するファイルの一式を作り、書き出し、コミット済みのもの（`docs/`）と比べる（osakidev/nokorazu-status の site.rs を写した）。
//!
//! 一式はサービスの一覧（`services.md`）だけから決まる（組み立てた時刻・環境の違いを含めない）。そのため、`docs/` が
//! 一覧と合っているかをテストと CI で確かめられる。

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::source::Content;
use crate::{DOMAIN, render};

/// CSS（先頭は company の guidelines.md 4.3 の写し）。
pub const STYLE: &str = include_str!("../../assets/style.css");

/// 組み立ての出力先であることを示す印（ブランチから配信するときに Jekyll を通さないためのファイルでもある）。
pub const MARKER: &str = ".nojekyll";

/// 配信するファイル（公開の場所からの相対パス → 中身）。ファビコンと RSS は作らない（README の 2 章）。
pub fn build(c: &Content) -> BTreeMap<String, Vec<u8>> {
    let mut m: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut put = |path: &str, body: String| {
        m.insert(path.to_string(), body.into_bytes());
    };
    put(MARKER, String::new());
    // 独自ドメインを付けるまでは CNAME を作らない（ブランチから配信に切り替えたときに、確認していないドメインを名乗らないため）
    if let Some(d) = DOMAIN {
        put("CNAME", format!("{d}\n"));
    }
    put("index.html", render::index(c));
    put("404.html", render::not_found(c));
    put("style.css", STYLE.to_string());
    m
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?.map(|e| e.map(|e| e.path())).collect::<Result<_, _>>()?;
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(&p, out)?;
        } else {
            out.push(p);
        }
    }
    Ok(())
}

/// `dir` の中のファイル（相対パス、`/` 区切り）。`dir` がなければ空。
pub fn list(dir: &Path) -> io::Result<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut v = Vec::new();
    walk(dir, &mut v)?;
    Ok(v.iter()
        .map(|p| p.strip_prefix(dir).unwrap_or(p).to_string_lossy().replace('\\', "/"))
        .collect())
}

/// 書き出す。一式にないファイル（独自ドメインをやめたときの CNAME など）は消す。
/// 消しすぎないように、出力先が空か、前に組み立てた印（`.nojekyll`）があるときだけ書く。戻り値は変わったファイル。
pub fn write(out: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<String>, String> {
    let existing = list(out).map_err(|e| format!("{}: {e}", out.display()))?;
    if !existing.is_empty() && !out.join(MARKER).exists() {
        return Err(format!(
            "{}: 組み立ての出力先ではないようです（{MARKER} がありません）。中のファイルを消さないように止めました",
            out.display()
        ));
    }
    let mut changed = Vec::new();
    for (rel, body) in files {
        let path = out.join(rel);
        if std::fs::read(&path).ok().as_deref() == Some(body.as_slice()) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))?;
        changed.push(rel.clone());
    }
    for rel in existing {
        if !files.contains_key(&rel) {
            let path = out.join(&rel);
            std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            changed.push(format!("{rel}（消した）"));
            // 空になったディレクトリも消す（out そのものは残す）
            let mut dir = path.parent().map(Path::to_path_buf);
            while let Some(d) = dir {
                if d == out || std::fs::remove_dir(&d).is_err() {
                    break;
                }
                dir = d.parent().map(Path::to_path_buf);
            }
        }
    }
    Ok(changed)
}

/// コミット済みの出力先と比べる。違いがなければ空。
pub fn diff(out: &Path, files: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let mut v = Vec::new();
    let existing = match list(out) {
        Ok(x) => x,
        Err(e) => return vec![format!("{}: 読めません: {e}", out.display())],
    };
    for (rel, body) in files {
        match std::fs::read(out.join(rel)) {
            Ok(b) if &b == body => {}
            Ok(_) => v.push(format!("{rel}: 中身が違います")),
            Err(_) => v.push(format!("{rel}: ありません")),
        }
    }
    for rel in existing {
        if !files.contains_key(&rel) {
            v.push(format!("{rel}: 組み立ての結果にない、余分なファイルです"));
        }
    }
    v
}
