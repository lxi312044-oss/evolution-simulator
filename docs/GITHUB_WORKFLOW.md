# GitHub運用（2026-11-06）

READMEのフェーズ合格条件・課題状態を維持しながら、次の仕組みを実装した。

## 変更の流れ

1. Issueテンプレートで目的、分類、状態、再現条件、合格条件を記録する。
2. 作業ブランチを作り、PRテンプレートで変更・検証・未検証事項を整理する。
3. CIと必要な実験結果を確認してmainへマージする。CI合格と科学的な進化実証を区別する。
4. フェーズ合格条件を満たした節目にタグを付ける。現時点は進化実証未達のため、フェーズ完了タグを自動発行しない。

Issue状態は「未着手・対応中・検証待ち・解決済み・先送り・継続観察」を本文に記録する。解決済みには根拠、先送りには理由と再確認フェーズを残す。

追跡中: [死亡Tickの観測不足 #1](https://github.com/lxi312044-oss/evolution-simulator/issues/1)、[草食絶滅と耐性進化の診断 #2](https://github.com/lxi312044-oss/evolution-simulator/issues/2)。

## 自動検証

`.github/workflows/ci.yml` はmainへのpush、PR、手動起動で3ジョブを独立実行する。PRブランチへのpushとPR更新で同じCIを二重起動しない。

| ジョブ | 検証 | 上限時間 |
|---|---|---|
| rust | 整形、Clippy全ターゲット、Rust全テスト、WASMコンパイル | 20分 |
| python | tools内の全Pythonテスト | 5分 |
| windows-regression | 固定10,000Tick回帰、1,000Tick×12条件の介入実験 | 25分 |

既存Rustの整形とClippy警告を修正した。警告の一括無効化はしない。旧ゲートテストはローカル実験結果の存在に依存していたため、専用fixtureで証拠あり・なしの両方を検証するようにした。本体のゲート条件を緩和していない。

ローカル検証: Clippy合格、Python25件合格、Rust全テスト合格、固定回帰は資源CSVのSHA-256一致・個体履歴の多重集合一致。GitHub側の結果は各PRのChecksを参照。

## 手動の長期比較

Actions → Manual intervention experiment → Run workflowで対象ブランチとTick数を指定する。1,000～50,000Tick、2seed×3環境×防御ON/OFFの12条件、100Tick間隔。毎pushで長期比較を実行しない。最大45分で停止し、途中停止は完了と扱わない。

結果の品質JSON、設定、実験来歴、集計CSVを7日保存する。全個体CSVをCI成果物に含めない。長期保存が必要な結論はdocsへ簡潔に記録する。科学的な成立判定には別途系譜・曝露等の評価が必要。

## 実験の来歴

`tools/run_phase0759.ps1` は実行前にビルドし、各ランの `.metadata.json` に以下を記録する。

- GitコミットSHA、dirty状態、Git差分対象の一覧。Git管理外の配布フォルダではnullで明示。
- 設定JSON、実行バイナリ、Rustソース・Cargo設定・実行補助のSHA-256。
- seed、Tick、間隔、UTC開始/終了日時、Rust/Cargo/Python版。
- 実行前はrunning、終了後に同じ入力であることを確認してcompleted。

completedは個別プログラムの正常終了であり、12条件の品質合格とは別。全体の終了コードとquality.jsonも確認する。失敗や中断はrunningのまま残る。設定やコードの途中変更を検知した場合は完了にしない。出力フォルダが空でなければ開始前に停止し、前回の証拠を保全する。

```powershell
python -m unittest discover -s tools -p 'test_*.py'
powershell.exe -ExecutionPolicy Bypass -File tools/run_phase0759.ps1 -Ticks 1000 -OutputDirectory results/my-new-run
```

## 保存と今後の設定

Gitにはソース・設定・報告・固定ベースラインを保存する。results、Pythonキャッシュ、ZIPは.gitignoreへ追加済み。ローカル25GB上限を継続し、生ログを旧版へコピーしない。

Webの画面検証・ビルド専用ジョブ、Projectsボード、必須レビューやmain保護は今回の実装対象外。CIジョブの実績を見てから必須チェックを設定する。main保護を設定していないため、PR経由は運用ルールであり強制ではない。

公式資料: [GitHub Actions設定・保存期間](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository)。
