#!/bin/sh
# 確かめ（一覧の決まり・ページの基本・CSP・リンク・外部の URL・個人情報・コントラスト・CSS・ワークフロー・docs/ が最新か）。
#   ./test.sh                                     # すべて
#   ./test.sh --test output コントラスト -- --nocapture   # コントラストの表を出す
set -eu
here=$(dirname "$0")
exec cargo test --locked --offline --manifest-path "$here/tools/Cargo.toml" "$@"
