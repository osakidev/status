#!/bin/sh
# サービスの一覧（services.md）から、配信するファイル（docs/）を組み立てる。
#   ./build.sh                 # 組み立てて docs/ に書く（終わったら docs/ もコミットする）
#   ./build.sh --check         # 書かずに、docs/ が一覧と合っているかだけ確かめる
#   ./build.sh --out 出力先     # 別の場所に組み立てる（Actions の配信）
# 一覧に誤りがあれば、何も書かずに止まり、直すところを行の番号つきで出す。
# 依存はないので、ネットワークにつながずに組み立てる（--offline）。crates.io にも頼らない。
set -eu
here=$(dirname "$0")
exec cargo run -q --locked --offline --manifest-path "$here/tools/Cargo.toml" --bin render -- "$@"
