//! サービスの一覧（services.md）の読み方。誤りは止めて、行の番号と直し方を出す。

use status_tools::source::{self, Service};

const GOOD: &str = "# 題\n> 覚え書き\n\n## サービス A\n説明: 例のためのサービスです。\n稼働状況: https://status.example.com/\n\n## サービス B\n稼働状況：https://example.github.io/b-status/\n説明：二つ目の例のサービスです。\n";

#[test]
fn 正しい一覧を読む() {
    let c = source::parse(GOOD).unwrap();
    assert_eq!(
        c.services,
        [
            Service { name: "サービス A".into(), desc: "例のためのサービスです。".into(), url: "https://status.example.com/".into() },
            Service { name: "サービス B".into(), desc: "二つ目の例のサービスです。".into(), url: "https://example.github.io/b-status/".into() },
        ],
        "並びは書いた順。全角の「：」も読む"
    );
}

#[test]
fn 本物の一覧を読める() {
    let c = source::load(&status_tools::repo_root()).unwrap_or_else(|e| panic!("{e:#?}"));
    assert!(!c.services.is_empty());
}

#[test]
fn 間違いを見つけて直し方を書く() {
    let ok = "## A\n説明: 例のサービスです。\n稼働状況: https://status.example.com/\n";
    let cases: [(String, &str); 18] = [
        (String::new(), "1 つもありません"),
        ("説明: 前に書いた。\n".to_string() + ok, "「## 名前」の行から"),
        (ok.replace("## A", "## "), "名前を書いて"),
        (ok.replace("## A", "## <b>"), "記号"),
        (format!("{ok}## a\n説明: 例です。\n稼働状況: https://status.example.org/\n"), "2 回あります"),
        (ok.replace("説明: 例のサービスです。\n", ""), "「説明: …」の行がありません"),
        (ok.replace("稼働状況: https://status.example.com/\n", ""), "「稼働状況: …」の行がありません"),
        (format!("{ok}説明: もう一つの説明です。\n"), "「説明」が 2 回"),
        (ok.replace("説明:", "紹介:"), "知らない見出し"),
        (format!("{ok}ただの文\n"), "の形で書いて"),
        (ok.replace("例のサービスです。", "例のサービスです"), "「。」で終えて"),
        (ok.replace("例のサービスです。", &format!("{}。", "あ".repeat(80))), "80 字まで"),
        (ok.replace("例のサービスです。", "安全なサービスです。"), "「安全」と書かない"),
        (ok.replace("例のサービスです。", "便利な道具です。"), "「道具」と書かない"),
        (ok.replace("https://", "http://"), "https:// から"),
        (ok.replace("example.com/", "example.com"), "「/」で終えて"),
        (ok.replace("https://status.example.com/", status_tools::SITE_URL), "このページ自身"),
        (ok.replace("例のサービスです。", "連絡は user@example.com へ。"), "メールアドレス"),
    ];
    for (text, want) in cases {
        let errors = source::parse(&text).expect_err(want);
        assert!(errors.iter().any(|e| e.contains(want)), "{want}: {errors:#?}");
        assert!(errors.iter().all(|e| e.starts_with("services.md: ")), "{errors:#?}");
    }
    // 電話番号・郵便番号は、覚え書きの行にあっても止める（公開のリポジトリに置かない）
    let errors = source::parse(&format!("> 連絡は 090-0000-0000 へ\n{ok}")).unwrap_err();
    assert!(errors.iter().any(|e| e.contains("電話番号")), "{errors:#?}");
    // 行の番号が付く
    let errors = source::parse(&ok.replace("説明:", "紹介:")).unwrap_err();
    assert!(errors[0].starts_with("services.md: 2 行目: "), "{errors:#?}");
}

#[test]
fn 誤りはまとめて出す() {
    let text = "## A\n紹介: x\n稼働状況: http://x/\n## \n";
    let errors = source::parse(text).unwrap_err();
    assert!(errors.len() >= 4, "{errors:#?}");
}
