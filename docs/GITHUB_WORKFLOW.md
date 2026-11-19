# GitHub運用（2026-11-06）

2026-11-06のユーザー指示により、**課題・計画・実施結果・研究判断・次の対応はGitHub Issuesを正本**とする。READMEへの進捗追記を終了する。科学的な合格条件と過去の履歴は維持する。

## 情報の置き場所

| 場所 | 役割 |
|---|---|
| Issues | 現在の課題・状態・実験計画/合格条件・結果/限界・判断・次の対応 |
| PR | 変更差分・レビュー・関連Issue・検証コマンドと結果リンク |
| Actions / Artifacts | 機械的な検証結果・保存期間付きの小さな成果物 |
| README | 概要・起動/導入・設計とIssuesへの案内。進捗台帳にしない |
| docs / PHASE文書 | 設計・固定仕様・長い補助証拠。可変の状態や結果の要約はIssueへ |
| ローカルresults | 生CSV・ハッシュ・再生成可能な出力。正本Issueに所在・保存方針を記録 |

最新の課題索引は[Issue #4](https://github.com/lxi312044-oss/evolution-simulator/issues/4)、Phase 07.5.9の実施結果と研究判定は[Issue #26](https://github.com/lxi312044-oss/evolution-simulator/issues/26)。過去のREADME課題表は当時の履歴として読む。新ルールは旧「READMEに固定/記録」の運用に優先するが、研究の合格条件は変更しない。

## 変更の流れ

### ローカルのGit管理

Git操作は `Cargo.toml` と `.git` があるリポジトリルートで行う。新しいPCでは既存リポジトリをcloneして履歴を取得する。ZIPを展開したフォルダだけでは履歴は取得できない。

```powershell
git clone https://github.com/lxi312044-oss/evolution-simulator.git
cd evolution-simulator
git switch -c work/issue-N-description
```

着手時は `git status --short` と `git fetch origin` で未コミット変更とリモートを確認する。作業中のファイルがある状態で `reset --hard` や強制checkoutを行わない。既存の配布フォルダをGit管理へ移す場合は、ファイルを保全してから既存リモートの履歴に接続し、ローカル差分を維持する。無関係な初期コミットで別履歴を作らない。

検証後は `git diff` と `git status` で対象を確認し、依頼・承認された範囲のファイルを明示してadd/commitする。pushは作業ブランチへ行い、mainへの反映はPR経由とする。最初のpushは `git push -u origin <作業ブランチ名>`。commitの前に作者名・メールが本人の設定になっていることを確認し、推測で登録しない。

非公開リポジトリをCLIから取得するには、Git Credential Manager等によるGitの認証が必要。アプリのGitHub連携とは認証が別の場合がある。トークンをorigin URL・ソース・ログへ埋め込まない。

`results/`、`target/`、`web/node_modules/`、生成物、キャッシュは既存の `.gitignore` で除外する。除外はバックアップではないため、実験証拠はハッシュとIssue上の所在を維持して別途保全する。Git管理を導入しても、過去の実験metadataのGit SHA nullを後から現在のSHAへ書き換えない。

1. 着手前にOpen/Closedを検索し、関連Issueを作成/更新する。目的、範囲、対照、評価指標、合格条件、再現条件、依存課題を固定し、対応中へ更新する。
2. 作業ブランチを作り、PRテンプレートで変更・検証・未検証事項を整理する。
3. 終了前にIssueへ実施日、SHA/ハッシュ、設定、seed、Tick、間隔、コマンド、結果/失敗、未検証事項、判断、状態、次の対応を記録する。PR/Actions/成果物をリンクして読み戻す。ローカル報告だけで終了しない。
4. CIと受入条件を確認してmainへマージする。CI成功と科学的な進化実証を区別し、研究課題を自動Closeしない。
5. フェーズ合格条件を満たした節目にタグを付ける。現時点は進化実証未達のため、フェーズ完了タグを自動発行しない。

Issue状態は「未着手・対応中・検証待ち・解決済み・先送り・継続観察」をタイトルと本文で揃える。実装が終わっても検証前は検証待ち。解決済みは根拠と受入条件の確認後Closed。先送り・継続観察はOpenとし、理由・再確認フェーズ/条件・次の対応を残す。履歴には日付と状態変更理由を追記する。

GitHub同期不能時はローカルに未同期下書きを一時保存し、最終報告でIssue番号・未反映内容・原因を明示する。更新成功を装わず、復旧後に反映・読み戻し確認する。長い報告をdocsに保存しても、要約・判定・次の対応をIssueに記録する。

外部更新の範囲はユーザーの依頼・既存承認に従い、承認/権限制約を迂回しない。

追跡中: [死亡Tickの観測不足 #1](https://github.com/lxi312044-oss/evolution-simulator/issues/1)、[草食絶滅と耐性進化の診断 #2](https://github.com/lxi312044-oss/evolution-simulator/issues/2)。

## 自動検証

`.github/workflows/ci.yml` はmainへのpush、PR、手動起動で3ジョブを独立実行する。PRブランチへのpushとPR更新で同じCIを二重起動しない。

| ジョブ | 検証 | 上限時間 |
|---|---|---|
| rust | 整形、Clippy全ターゲット、Rust全テスト、WASMコンパイル | 20分 |
| python | tools内の全Pythonテスト | 5分 |
| windows-regression | 固定10,000Tick回帰、1,000Tick×12条件の介入実験 | 25分 |

既存Rustの整形とClippy警告を修正した。警告の一括無効化はしない。旧ゲートテストはローカル実験結果の存在に依存していたため、専用fixtureで証拠あり・なしの両方を検証するようにした。本体のゲート条件を緩和していない。

導入時の検証記録はPR #3のChecksを参照。以後の各実行の検証数・結果・未達条件は対応Issueへ記録し、この運用文書に最新の件数を複製しない。

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
