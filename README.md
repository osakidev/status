# status — OSAKI Dev の稼働状況のまとめのページ

- **状態:** 2026-11-23 に作り、同じ日に公開のリポジトリ osakidev/status を作って GitHub Pages で公開した（https://osakidev.github.io/status/）。独自ドメイン status.osaki.dev は、osaki.dev の DNS を Route 53 に移した後に付ける（company#65）。追跡は osakidev/nokorazu#164
- **決まっていること（代表、2026-11-23）:** 中身は「OSAKI Dev のサービスと、それぞれの稼働状況のページへのリンク」だけ。リポジトリの名前は `status`（会社のもの・共通のものは頭に何も付けない。company の `docs/company/12-naming.md`）。公開のリポジトリにし、GitHub Pages で出す。独自ドメインは将来 `status.osaki.dev`（`osaki.dev` の DNS を Route 53 に移すとき。5.2）
- **従う決まり:** osakidev/company の `docs/design/guidelines.md` 版 1.1（文章は 3.、色は 4.、確かめ方は 13.）。強調色は会社の色の浅葱（4.1 と 14. の 1）。アクセシビリティの目標は JIS X 8341-3:2016 AA・WCAG 2.2 AA
- **もとにしたもの:** osakidev/nokorazu-status（6c70dd6）の作りと決まりを小さく写した。違うところは [9 章](#9-nokorazu-status-から変えたこと写したもの)

## 目的

OSAKI Dev（大崎開発）のサービスごとの稼働状況のページを、1 か所からたどれるようにする。
**このページはサービスの状態を持たない。** 状態とお知らせは、サービスごとの稼働状況のページ（`<サービス>-status`）だけに書く。
二重に書くと、障害のときに直す場所が 2 つになり、食い違うため。

## 1. 構成

```
services.md              サービスの一覧（名前・短い説明・稼働状況のページの住所）。ここだけを直す（3 章）
docs/                    配信するファイル（組み立ての結果。コミットする。4 章）
  index.html             一覧のページ
  404.html               見つからないとき（一覧のサービスへのリンクも載せる）
  style.css              見た目（assets/ の写し）
  .nojekyll              ブランチから配信するとき（予備の道）に使う印
assets/style.css         見た目（先頭はガイドライン 4.3 の写し。強調は浅葱）
tools/                   道具（Rust の標準ライブラリだけ。外部クレートなし）
  src/source.rs          services.md を読み、決まりどおりか確かめる（誤りの文は日本語で、直し方つき）
  src/render.rs          HTML のページ
  src/site.rs            一式を作る・書き出す・docs/ と比べる
  src/html.rs src/css.rs src/contrast.rs   確かめ用の小さな解析とコントラスト比
  src/bin/render.rs      ./build.sh の中身
  tests/                 確かめ（7 章）
.github/workflows/pages.yml   確かめと配信（4 章）
.github/dependabot.yml        固定したアクションの版を週 1 回見る
SECURITY.md              脆弱性の連絡の案内
build.sh test.sh
```

- ページはスクリプト（JavaScript）・画像・外部のフォント・CDN・解析・埋め込み・Cookie・フォームを使わない
- ページの中のリンクは相対（`style.css`・`./`）。独自ドメインがない今の `https://osakidev.github.io/status/` でも、`status.osaki.dev` を付けた後も、そのまま開ける。404.html だけは、どの URL でも出るので、公開の場所のパス（今は `/status/`）から書く（`tools/src/lib.rs` の `SITE_URL`）
- 組み立ては「今の時刻」を使わない。同じ一覧からは、いつ組み立てても同じ一式になる（だから docs/ が最新かをテストで確かめられる）
- ネットワークも依存も使わない（`cargo --locked --offline`）

## 2. 作らないもの

| もの | 理由 |
|---|---|
| サービスの状態・お知らせ・RSS | 状態はサービスごとのページが持つ（目的）。お知らせがないので RSS も作らない。受け取りたい方は、サービスごとのページの RSS を使う |
| ファビコン・会社の印 | 会社の印がまだ決まっていない（osakidev/site の README の「ロゴ（マーク）」は仮の案 D で、商標の調査が残っている）。ヘッダーは文字だけにした。決まったら `assets/` に足し、CSP に `img-src 'self'` を足す（`tools/src/render.rs` の `CSP`、テストの `ページの基本` の `img`・`svg` の禁止も直す） |
| 開発用サーバー・ブラウザーでの確かめの道具 | ページが 1 枚で、`nokorazu-status` の `serve.sh` と `screenshots.sh` を写すほどでないため。見た目は 7 章の「まだ確かめていないこと」 |
| サービスの本体（`nokorazu.jp` など）へのリンク | 中身は稼働状況のページへのリンクだけ（代表の決定）。公開の前のサービスへリンクしないためでもある |

## 3. サービスを足すとき

### 3.1 書き方（`services.md`）

```
## nokorazu
説明: ファイルを送る前や公開する前に、見えないのに取り出せてしまう情報が残っていないかを確かめるサービスです。
稼働状況: https://status.nokorazu.jp/
```

| 行 | 要るか | 書くこと |
|---|---|---|
| `## 名前` | 必ず | サービスの名前（30 字まで）。ここから 1 つのサービス。並びはページの並び |
| `説明:` | 必ず | 1 文（80 字まで、「。」で終える）。「……するサービスです。」。リンク・タグは書けない（文はそのまま出す） |
| `稼働状況:` | 必ず | そのサービスの稼働状況のページ。`https://` で始め、`/` で終える。`?`・`#` は付けない |

- 先頭の `# ` の題と `>` の覚え書きは、公開しない（ページに出ない）。ほかの文を最初のサービスの前に書くと止まる
- 全角の `：` でも読む
- **止まるもの:** 知らない見出し、同じ見出し・同じ名前が 2 回、`説明`・`稼働状況` がない、`https://` でない・`/` で終わらない住所、このページ自身を指す住所、80 字を超える説明、「安全」「完全」「100%」「絶対」「株式会社」「道具」（ガイドライン 3.5、サービスを「道具」と呼ばない決まり）、メールアドレス・電話番号・郵便番号・郵便の記号（覚え書きの行も）。誤りは全部、行の番号と直し方つきで出し、何も書かない（一覧は `tools/tests/source.rs` の「間違いを見つけて直し方を書く」）

### 3.2 手順

1. 先に、そのサービスの稼働状況のページ（`osakidev/<サービス>-status`）を公開する（company の `docs/company/12-naming.md`。`osakidev/nokorazu-status` を写して作る）。HTTPS で開けることを確かめる
2. `services.md` に 3 行を足す
3. 手元（`~/claude/osakidev/services/status`）で:

```sh
./build.sh            # 確かめて docs/ に組み立てる。誤りがあれば止まる
./test.sh             # 確かめ（docs/ が最新かも）
git add services.md docs
git commit -m "一覧: <サービス> を足す（<Issue>）"
git push              # main に入れるのは親。ブランチで作り、merge する
```

4. Actions の「確かめと配信」が緑になったら、公開の場所を開いて、足したサービスが出ていることを確かめる

GitHub のサイトで `services.md` だけを直してコミットすると、docs/ が古いままなので `test` が赤になり、配信されない（4 章）。急ぐものではないので、Mac で組み立ててからコミットする。

## 4. 組み立てと配信

**組み立てた結果（docs/）はコミットし、テストで最新かを確かめる。配信は、確かめが通った後に、一覧から組み立て直した結果を GitHub の公式のアクションで配る（Pages の Source は「GitHub Actions」）。予備として、docs/ をそのまま配る「ブランチから配信」に、設定で切り替えられる。** `nokorazu-status` の 4 章と同じ方式（案 B）。

- **`nokorazu-status` と違うところ: 配信は確かめ（test）を待つ**（test → build → deploy）。`nokorazu-status` は、障害のお知らせをテストの都合で止めないように待たない。このページには急いで載せるものがないので、確かめが通らないものは配らない方を取った
- **生成物をリポジトリに含める理由:** 公開する HTML の差分がコミットで見える。公式のアクションや Rust の都合で Actions の組み立てが動かなくなっても、予備の道でそのまま配れる
- **アクションの版:** `actions/checkout` v7.0.1、`actions/upload-pages-artifact` v5.0.0、`actions/deploy-pages` v5.0.1（`nokorazu-status` と同じ）。SHA は 2026-11-23 に `gh api repos/actions/<名前>/git/ref/tags/<版>` で本物のタグと一致することを確かめた。3 つとも、その日の最新のリリースだった。Dependabot が週 1 回、上げる PR を作る
- **権限:** 既定はなし（`permissions: {}`）。`test`・`build` は `contents: read`、`deploy` だけ `pages: write` と `id-token: write`。pull request では配信しない。シークレットはない
- **Rust:** Ubuntu のイメージに入っているものを使う（`ubuntu-24.04`。外部のアクションとダウンロードを増やさないため）。手元は 1.99.1。イメージが `rust-version`（1.88）より古くなるか Rust を外すと止まる
- **予備の道への切り替え:** Settings → Pages → Source を「Deploy from a branch」・`main`・`/docs` にし、Settings → Secrets and variables → Actions → Variables に `STATUS_PAGES_SOURCE` = `branch` を足す（配信のジョブが止まり、確かめは続く）。戻すときは逆にする
- **大きさ:** 1 回の表示は約 8 KB（HTML と CSS）

## 5. 公開の手順

### 5.1 GitHub で（代表・親）

| # | すること | 場所 | 状況 |
|---|---|---|---|
| 1 | **nokorazu の稼働状況のページが HTTPS で開けることを確かめる。** このページは `https://status.nokorazu.jp/` へリンクする。2026-11-23 の時点では証明書がまだない（`gh api repos/osakidev/nokorazu-status/pages --jq .https_certificate` が `null`）。証明書ができる前に公開すると、リンク先で証明書の警告が出る | `osakidev/nokorazu-status` の 8.1 の 8 | 残り |
| 2 | **公開のリポジトリ `osakidev/status` を作る。** README などは作らない（空のまま）。Wiki はオフ。**作ると、古い名前 `osakidev/status` から `osakidev/nokorazu-status` への転送が止まる**（company の `docs/company/12-naming.md`）。古いリンクはこのリポジトリを指すが、ページと README から nokorazu の稼働状況へたどれる | GitHub | 残り |
| 3 | **Pages の配信元を「GitHub Actions」にする。** 空のリポジトリで選べないときは 4 のあとに（最初の配信が失敗するだけで、選んだあと Re-run すればよい [推測]） | osakidev/status → Settings → Pages | 残り |
| 4 | **push する。** 押す前に、履歴の作者（6.3）とテストを確かめる。`git remote add origin git@github.com:osakidev/status.git`、`git push -u origin main` | 手元 | 残り |
| 5 | **配信を確かめる。** Actions の「確かめと配信」の `test`・`build`・`deploy` が緑の印になったら、`https://osakidev.github.io/status/` と、ない住所（`https://osakidev.github.io/status/x`）を開く | ブラウザー | 残り |
| 6 | **守りの設定**（`nokorazu-status` の 8.1 の 9 と同じ）: `main` の強制プッシュと削除を禁止（ルールセット）。環境 `github-pages` の配信を `main` だけに。Private vulnerability reporting をオン（SECURITY.md の案内が使えるように）。secret scanning と push protection をオン。Wiki と Discussions をオフ。トピック `company` | Settings | 残り |
| 7 | **company の文書を直す:** `docs/security/github.md` の 2-7・4-1・4-9・4-10 の例外に `status` を足す。`docs/infra.md` の「稼働状況のページ」と `docs/company/12-naming.md` の「`status`（`status.osaki.dev`。まだない）」を、作ったことに直す。`docs/infra.md` のデータの置き場所の例外（GitHub の IP の記録）に、このページも含める | company | 残り |

### 5.2 独自ドメイン `status.osaki.dev`（`osaki.dev` の DNS を Route 53 に移すとき）

今の `osaki.dev` の DNS は Squarespace にある（company の `docs/infra.md`）。Route 53（`osakidev-common` のゾーン）に移すときに、次の順で付ける。`nokorazu-status` の 7 章・8.1 の 4〜8 と同じ手順。

1. **組織で `osaki.dev` を確認する。** 組織 → Settings → Pages → Add a domain に `osaki.dev`。表示された TXT（`_github-pages-challenge-osakidev.osaki.dev`、TTL 300）を Route 53 に作り、`dig _github-pages-challenge-osakidev.osaki.dev TXT` で見えたら Verify。**確認のあとも TXT は消さない**（消すと確認が外れる）
2. **道具を直す。** `tools/src/lib.rs` の `SITE_URL` を `https://status.osaki.dev/` に、`DOMAIN` を `Some("status.osaki.dev")` にする。`./build.sh`（`docs/CNAME` ができ、404.html のリンクが `/` からになる）、`./test.sh`、コミット、push
3. **Custom domain を入れる。** 2 の配信が終わったらすぐ、Settings → Pages → Custom domain に `status.osaki.dev` を入れて Save（Actions で配信するときは、ここでの設定がすべて。CNAME のファイルは予備の道のため）。2 と 3 の間は、404.html の見た目だけが崩れる（一覧のページは相対のリンクなので崩れない）
4. **そのあとで** Route 53 に CNAME `status.osaki.dev` → `osakidev.github.io`（リポジトリ名は付けない。TTL 3600）を作る。**先に DNS を作らない**（ほかの人がその名前でページを出せてしまう）
5. DNS の確認が通ったら「Enforce HTTPS」を選ぶ（証明書ができるまで時間がかかることがある）
6. 確かめ: `gh api repos/osakidev/status/pages --jq '{cname,https_enforced,https_certificate}'`、`dig +short status.osaki.dev CNAME`（`osakidev.github.io.`）

- ワイルドカード（`*.osaki.dev`）のレコードは作らない。CAA を使うなら `0 issue "letsencrypt.org"` を足す
- Pages を止めるとき・リポジトリを消すときは、先に CNAME を消す（残すと乗っ取りの危険）

## 6. セキュリティとプライバシー

### 6.1 CSP と、効かないもの

GitHub Pages は HTTP のヘッダーを足せないので、CSP は各ページの先頭の `<meta http-equiv="Content-Security-Policy">` に書いた。

```
default-src 'none'; style-src 'self'; base-uri 'none'; form-action 'none'
```

画像（ファビコンを含む）を使わないので、`nokorazu-status` と違って `img-src` も書かない。`frame-ancestors`・`report-uri`・`sandbox` は meta では効かず、`X-Frame-Options`・`X-Content-Type-Options`・`Permissions-Policy` なども付けられない。押すと何かが起きる部品がなく、扱う情報もないので、影響は小さいと考える [推測]（細部は `nokorazu-status` の 6.1）。`<meta name="referrer" content="no-referrer">` でリンク先に参照元を送らない。

### 6.2 プライバシー

- ページは Cookie・スクリプト・アクセス解析・外部からの読み込みを使わない
- 配信している GitHub は、見に来た人の IP アドレスをセキュリティのために記録して保存する（GitHub Docs「About GitHub Pages」の Data collection）。運営者は受け取らない。ページの「このページについて」に、`nokorazu-status` と同じ言い方で書いた
- データを日本だけに置く方針の例外は、`nokorazu-status` と同じ扱い（公開の文だけ。IP の記録は GitHub のもの）。company の `docs/infra.md` の例外にこのページも含める（5.1 の 7）

### 6.3 公開のリポジトリ

- コミットの作者の名前とメールアドレスも公開される。このリポジトリの `git config` は、作者を今のアカウント名と GitHub の noreply のアドレスにしてある（`nokorazu-status` と同じ）。押す前に `git log --all --format='%an <%ae> %cn <%ce>' | sort -u` で確かめる
- この README も公開される。秘密は書いていない（鍵・アカウントの番号・個人のアドレスなし）。社内のリポジトリの名前と Issue の番号は出る
- 個人情報を置かない: 運営の表示は「OSAKI Dev（大崎開発）」だけ。`tests/repo.rs` が、メールアドレス・電話番号・郵便番号・郵便の記号・古いアカウント名がないかをすべてのファイルで見る
- ライセンスは付けない（`nokorazu-status` の 9 章の 16 と同じ）

### 6.4 GitHub の設定

company の `docs/security/github.md` のうち、`nokorazu-status` と同じく公開のリポジトリなので、4-1（private）と 4-9（fork の禁止）は当てはまらず、4-10 の Pages は使う。組織の 2-7（Pages は Public だけ許可）は、会社全体の `status` も対象に入れて決めてある。

## 7. 確かめ

```sh
./test.sh                                          # すべて
./test.sh --test output コントラスト -- --nocapture   # コントラストの表
cargo clippy --manifest-path tools/Cargo.toml --all-targets --locked --offline -- -D warnings
```

| テスト | 確かめること |
|---|---|
| `tests/output.rs` | HTML の基本（lang・charset・viewport・referrer・title・description・h1・ランドマーク・本文へ移動・見出しの順・id の重なり・スクリプトや画像や style 属性や新しいタブを使わない・自動で読み込み直さない・読み込むのは style.css だけ）、CSP（charset の次の meta、値は `'self'` と `'none'` だけ）、中のリンクの行き先、使われていないファイルがない、外部のリンクは一覧のサービスと許可したものだけ、一覧のサービスがすべて並びどおりに出る、記号は文字のまま出る、状態の言葉を書かない、読み上げの順、プライバシーの説明と運営者、404.html のリンク、大きさ、書かない言葉、メールアドレス・電話番号・住所、コントラスト（ライト・ダーク・印刷）、CSS の決まり（色は変数だけ・rem・余白 7 段・固定表示なし）、**docs/ が一覧と合っている**、同じ一覧からいつも同じ一式 |
| `tests/source.rs` | 一覧の読み方。間違い 18 種で止まり、直し方の文と行の番号が出る。誤りはまとめて出る。全角の `：` を読む |
| `tests/repo.rs` | 依存ゼロ、スクリプトがネットワークを使わない、ワークフロー（公式のアクションを SHA で固定・権限は最小・シークレットなし・AWS なし・配信は確かめの後）、公開のリポジトリに個人情報を置かない |
| 単体 | 住所の決まり、個人情報の見つけ方、公開の場所と独自ドメインの組み合わせ、HTML・CSS の解析、コントラストの式 |

**まだ確かめていないこと:**

- ブラウザーでの見た目（幅 320px・文字 200%・ダーク・印刷・強制カラー）、`<meta>` の CSP が実際に効くこと、Tab の動き。`nokorazu-status` は `screenshots.sh` でヘッドレス Chrome で見たが、このページでは見ていない。CSS の組み方は `nokorazu-status` と同じ部品だけを使った
- スクリーンリーダー（VoiceOver・NVDA）での読み上げ。`role="list"` は、`list-style: none` の一覧を Safari が一覧と伝えないことがある問題への手当てだが、実物では確かめていない [推測]
- GitHub の上での動き（ワークフロー・Pages・404.html の出方）。リポジトリがまだないため（5.1）

## 8. 出典と確認日

- GitHub Docs「About GitHub Pages」（Data collection）、「Configuring a publishing source」、「Managing a custom domain」、「Verifying your custom domain」、「Securing your GitHub Pages site with HTTPS」: `nokorazu-status` の 11 章（2026-11-04 に読んだ）。この作業では読み直していない
- アクションの版と SHA、組織の Pages の状態: `gh api`（読み取りだけ。2026-11-23）
- 会社の決まり: company の `docs/design/guidelines.md` 版 1.1、`docs/company/12-naming.md`（2026-11-23 の更新）、`docs/infra.md`

## 9. nokorazu-status から変えたこと・写したもの

| 変えたこと | 理由 |
|---|---|
| 文のファイルは `services.md` 1 つだけ。状態・お知らせ・窓口・型・例・RSS をなくした | 中身はサービスとリンクだけ（代表の決定）。状態を二重に書かない |
| 配信は確かめを待つ | 急いで載せるものがない（4 章） |
| 一覧の誤りは注意で済ませず、止める | 同じ理由。障害のときの公開を止める心配がない |
| ファビコンと画像をなくし、CSP の `img-src` を外した | 会社の印が決まっていない（2 章） |
| 強調色を浅葱に。状態の色（エラー・警告・成功）は写していない | 会社のページなので会社の色（ガイドライン 4.1）。状態を出さない |
| 独自ドメインがない間は CNAME を作らず、404.html のリンクを `/status/` から書く | `status.osaki.dev` はまだない（5.2） |
| 開発用サーバーとブラウザーの確かめの道具を写していない | 小さく保つため（2 章） |

| もの | 元 | 作った人 |
|---|---|---|
| `assets/style.css` の色・文字の定義 | company の `docs/design/guidelines.md` 4.3、4.1（浅葱） | 大崎開発 |
| `tools/src/` の html・css・contrast、`site.rs` の書き出しと比べ方、`source.rs` の個人情報の見つけ方、テストの多く | `osakidev/nokorazu-status`（元は osakidev/site） | 大崎開発 |
