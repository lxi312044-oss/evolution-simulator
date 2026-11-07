# シミュレーション再検証（2026-11-06）

## 対象と判定

現行ローカルのGitHub運用実装版を再検証した。対応するPRは [#3](https://github.com/lxi312044-oss/evolution-simulator/pull/3)、headは `c34481b017547507e357687c24ca7a47e48daee2`。ローカルは.gitのない配布フォルダなので、各ランはGit SHAをnullとしてソースとバイナリのハッシュを記録している。

**実施した技術検証はすべて合格。** 生態モデルの修正は行っていない。草食絶滅と遺伝的進化の実証は引き続き別の研究課題であり、Phase 08移行を承認する結果ではない。

## 今回実行した検証

| 項目 | 結果 |
|---|---|
| Rust全テスト（verify_phase0733.ps1内） | 93件合格、失敗0 |
| Python全テスト | 25件合格、失敗0 |
| cargo fmt --all -- --check | 合格 |
| cargo clippy --locked --workspace --all-targets -- -D warnings | 合格 |
| cargo check --locked -p sim-wasm --target wasm32-unknown-unknown | 合格 |
| 固定旧版回帰、seed12345、10,000Tick、100Tick間隔 | 資源・繁殖CSVのSHA-256一致、個体履歴全レコード一致（行順を除外し重複数も比較） |
| 介入実験を新規実行 | 10,000Tick×12条件、100Tick間隔。全ラン正常完走、品質PASS |
| 過去の同条件実験と比較 | types/births/outcomes/individualsの48 CSVが全レコード一致 |
| 実験来歴 | 12件すべてcompleted。開始・終了間の入力一致を確認、検証後も全ソースハッシュ一致 |
| wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg | 合格 |
| npm run build | TypeScript検査・Vite本番ビルド合格 |
| 生成WASMの実行 | 通常毒ON/OFF各10,000Tick×2反復。全Tick正常、同条件の最終スナップショット・診断が完全一致 |

GitHubの [CI実行42168197113](https://github.com/lxi312044-oss/evolution-simulator/actions/runs/42168197113) も読み取り確認し、rust/python/windows-regressionの3ジョブ成功を確認した。このCIは前回PR作成時の実行であり、今回新しく再実行したCIではない。

## 実験条件と結果

介入実験: seed12345/12346 × 初期毒A割合0.8/0.2/0.5 × 防御効果0.8/0。共通耐性維持費倍率0.5、追加タイプ倍率1。既存の `run_phase0759.ps1` で実行。12ラン計120,000Tickで、各Tickの既存資源保存チェックがエラーなく通った。

| seed | 初期毒分布 | 最終草食数 ON / OFF |
|---|---|---:|
| 12345 | A80 | 0 / 0 |
| 12345 | B80 | 0 / 0 |
| 12345 | equal | 91 / 20 |
| 12346 | A80 | 0 / 3 |
| 12346 | B80 | 34 / 0 |
| 12346 | equal | 23 / 16 |

草食絶滅は再び12ラン中6ラン。過去の結果を完全再現しており、今回の整形・開発基盤変更による新しい退行は観測されなかった。絶滅個体群の耐性は欠測として扱う。2seedの結果だけで毒タイプ別の適応進化を証明しない。

WASM実行: `config/standard-toxin.json`、seed12345、毒enabledのみON/OFF、標準自然発芽を有効化。100Tickごとに資源誤差を独立確認し、10,000Tickの全最終スナップショットを2反復で比較した。介入実験とは異なる単一毒・自然発芽ありの設定。

| 毒 | 最終 植物 / 草食 / 肉食 | 最大栄養誤差 | 最大エネルギー誤差 |
|---|---|---:|---:|
| OFF | 302 / 245 / 2 | 3.45e-9 | 1.08e-4 |
| ON | 1638 / 306 / 5 | 2.94e-9 | 1.13e-4 |

WASM側の独立チェックは栄養誤差1e-3以下、エネルギー誤差は既存の絶対/相対許容誤差以下を確認。World内部は各Tickで保存則を検査する。画面描画、操作、ブラウザ固有の動作は今回未検証であり、Node上のWASM実行とWebビルドの合格からUIの合格を推論しない。

## 記録と再現

- 出力: `results/revalidation-20261106/verification.json`、`wasm-verification.json`、各検証ログ、`interventions/`。
- 出力間隔は100Tick。全Tickの詳細ログは保存しない。介入実験の出力は約35.1MB。
- 実行ソースmanifest（パスとSHA-256の辞書をキー順JSON化してSHA-256）: `dae03e8e0ff558b095ead6f09c9a8744a08935c4704d5e0a36f5227bdf9b4e52`。
- Rust 1.99.1、Cargo 1.99.1、Python 3.14.7、Node 24.13.1、npm 11.8.0、Vite 6.4.5。
- Web検証のため依存を取得し、`web/package-lock.json` と生成物を作成。依存の更新検証やGitHubへのpushは今回行っていない。
- 最終容量確認: 約1.67GB、運用上限25GB以内。

## 残る課題とGitHub状態

- 死亡Tick内の計測不足: [Issue #1](https://github.com/lxi312044-oss/evolution-simulator/issues/1)。CSVの再現一致は、その計測の生物学的完全性を保証しない。
- 草食絶滅・耐性進化の診断: [Issue #2](https://github.com/lxi312044-oss/evolution-simulator/issues/2)。多seed、系譜、曝露、死因の追加評価が必要。
- PR #3は今回確認時点でopen・未マージ。前回マージ操作は自動承認審査が利用上限で完了できず実行されていない（安全性を理由とする拒否ではない）。今回の依頼は再検証として実施し、マージ操作は再試行していない。
