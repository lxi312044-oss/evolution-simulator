# Phase 06.2.6 — 草食動物保護の効果検証

## 目的
既存の自然調整（06.2.5）を固定し、草食動物保護だけを変更して、同じ30シードで比較する。**標準ONへの変更は行わない**。

| 条件 | 保護 | 閾値 |
|---|---|---:|
| off | OFF | 20（無効） |
| refuge20 | ON | 20 |
| refuge50 | ON | 50 |
| refuge100 | ON | 100 |

## 実行（プロジェクトルート）
```powershell
cargo test --workspace
.\run_phase0626.ps1
```
短時間の動作確認: `.\run_phase0626.ps1 -Ticks 1000 -SeedCount 2`

結果は `results/phase0626/<条件>/per_seed.csv` と `summary.csv` に保存する。各条件のシードは同一で、デフォルトは12345〜12374。個別のエラーはper_seed.csvに残す。失敗シードは共存率の分母から除外し、必ず失敗数を併記する。

### 評価
- `three_species_survival_rate`: **50,000 Tick時点の3種生存率**。一度絶滅して後から復活する機構がない前提。
- `herbivore_survival_count` と `carnivore_survival_count`: 保護がどちらの生存に影響するか。
- `total_refuge_blocks`: 保護判定が実際に発動したか。
- `herbivore_min`: サンプリング時点の最小草食動物数。
- `max_abs_nutrient_error` と `max_abs_energy_error`: N/E保存誤差（サンプリング時点での最大）。
- `*_extinction_tick`: 記録間隔内での初回0観測時点。正確な死亡Tickではない。

### 判定の注意
1シードでの効果を一般化しない。保護が発動しない条件では「効果なし」と断定しない。草食動物の生存率が改善しても肉食動物の生存率が悪化する可能性がある。標準ONは比較結果を確認してから判断する。閾値100は意図的な強い介入であり、自然な進化結果を歪める可能性に注意。

### 互換性
06.2.5のロジックは変更せず、設定4件・評価用Rust example・PowerShell実行スクリプトのみを追加。既存の設定ファイルは変更しない。
