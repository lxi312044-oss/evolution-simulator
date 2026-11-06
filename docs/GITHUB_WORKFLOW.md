# GitHub活用案（2026-11-06）

この文書は今後の運用案。既存READMEのフェーズ合格条件・課題状態を変更しない。Issue、Projects、ブランチ保護などの設定はまだ追加していない。

## 推奨する進め方

1. 課題ごとにIssueを作り、目的、再現手順、seed、設定、Tick、期待結果、合格条件を記録する。バグ、観測不足、科学的仮説を区別する。
2. 1課題につき作業ブランチとPRを作る。PRに変更理由、検証コマンド、結果、未検証事項を記載する。小さな解析修正と生態モデル変更を分ける。
3. 自動検証と必要な実験結果を確認してmainへマージする。フェーズ完了判定はREADMEの科学的条件も満たしたときだけ行う。
4. 検証済みの節目にタグを付ける。実験結果にはコードのコミットSHA、設定、seed、Tick、観測間隔、ツールチェーン版を記録し、どの版の結果か追跡できるようにする。

## 自動検証の優先順位

既存 `.github/workflows/ci.yml` はRustの整形、Clippy、テスト、WASMコンパイル確認を定義している。ただし2026-11-06のローカル確認では `cargo fmt --all -- --check` が既存Rustソースで失敗した。後続ステップに到達しない問題を、専用PRで解消するのが先決。Clippyの合格は今回確認していない。

次にPython解析テスト `python -m unittest discover -s tools -p test_phase0759.py` を独立したジョブに追加する。Rustの整形エラーがあっても解析の検証結果を見られる構成にする。

Windows向けPowerShellの短期12ラン、旧版10,000Tick回帰は別ジョブで実施し、実行時間を確認してからPR必須チェックにする。数万Tick×多数seedの長期実験は手動実行から始め、毎pushで起動しない。Web検証はWASM生成とTypeScript/Viteビルドの手順を別途整備する。

## 課題の初期候補

| 優先 | 課題 | 完了の目安 |
|---|---|---|
| 1 | CIの整形失敗と後続検証の確認 | 整形・Clippy・Rustテスト・WASM確認が実際に成功 |
| 2 | Python解析のCI追加 | 不完全入力を拒否する12テストがPRごとに実行される |
| 3 | 死亡Tick内の計測欠落 | 同Tickの摂取・繁殖・死亡をイベントで検証できる |
| 4 | 実験の来歴記録 | 各ランにコミットSHA・設定ハッシュ・実行条件を保存 |
| 5 | 草食絶滅と耐性進化の追加診断 | 死因・曝露・系譜と対照条件から仮説を評価 |

Issueの状態はREADMEと同じ「未着手・対応中・検証待ち・解決済み・先送り・継続観察」を使う案とする。未解決の観測課題や科学的仮説は、実装が終わっただけで閉じない。

## 容量と成果物

Gitにはソース・設定・簡潔な検証報告・必要な固定ベースラインを保存する。`target`、`node_modules`、実行ログ、全実験CSV、配布ZIPは通常のコミット対象にしない。現行 `.gitignore` は `results/` とPythonキャッシュを除外していないため、今後ローカルGitを導入するときに見直す。

CI成果物は品質JSON・小さな集計・失敗診断を中心にし、保存期間はまず7日を提案する。長期保存する検証報告はリポジトリ内に残す。GitHub側の容量と実行時間も確認し、ローカル25GB上限の運用も継続する。

公式資料: [GitHub Actionsの設定と保存期間](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository)。実際の保存期間・権限設定は導入時に確認する。
