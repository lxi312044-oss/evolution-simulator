# Phase 07.5.3 — 毒タイプ別耐性の世代進化観測（計測実装）

## 目的
Phase 07.5.2で導入したタイプ別負荷と遺伝性耐性について、植物毒タイプの存在数、草食耐性分布、負荷、出生時の親子遺伝、死亡時の形質・繁殖成功をタイプ別に記録する。**自然選択が成立したかの最終判定はCSV分析後**。

## 変更範囲
- `crates/sim-core/examples/toxin_evolution_0753.rs` の読み取り専用計測例のみ。生態シミュレーション本体、乱数、毒物理、資源会計は変更しない。
- `tools/run_phase0753.ps1` と `tools/verify_phase0753.ps1`、2 seed のパイロット設定を追加。
- `*.types.csv`: 各タイプの植物数、草食耐性の平均・分散・0.5以上の比率、毒負荷、出生親平均、出生親平均との差（暫定選択差）、出生・死亡数と形質。
- `*.births.csv`: 実際の出生イベントに紐づく親子ID、タイプ別親子耐性、差分。
- `*.outcomes.csv`: 死亡個体のタイプ別耐性、年齢、累積繁殖成功、残存負荷。

## 実行
```
powershell.exe -ExecutionPolicy Bypass -File ".\tools\verify_phase0753.ps1"
powershell.exe -ExecutionPolicy Bypass -File ".\tools\run_phase0753.ps1" -Config ".\docs\phase07\phase0753-pilot-seed12345.json" -Ticks 10000 -Interval 100 -OutputPrefix "results/phase0753/seed12345"
powershell.exe -ExecutionPolicy Bypass -File ".\tools\run_phase0753.ps1" -Config ".\docs\phase07\phase0753-pilot-seed12346.json" -Ticks 10000 -Interval 100 -OutputPrefix "results/phase0753/seed12346"
```

## 判定条件
1. Rust全テスト・Phase 07.3.3旧版回帰PASS。
2. 両seedでCSV生成。各tickの植物毒タイプ数・耐性分布が有限で再現性があること。
3. 親子CSVでタイプ別の遺伝差と変異が追跡できること。
4. 進化成立判定は別途：初期/最終差だけでなく時系列、出生選択、死亡選択、環境タイプ分布を合わせて確認。ランダムドリフト・生態変動と区別できない場合は「未確定」。

## 限界と解釈
- `selection_differential` は当該窓の出生親平均と窓末時点の生存個体平均の差。厳密な世代別適応度・因果的選択係数ではない。
- `mutation_observed` は親子遺伝値の相違であり、内部の突然変異試行フラグではない。丸めやクランプで変異試行が見えない場合がある。
- 出生イベントで親または子が同一tick内に死亡した場合、出生親子CSVに記録されないことがある。死亡CSVはtick境界での消失を記録。
- 種別耐性は連続値（0〜1）であり、`high_resistance_fraction` は0.5以上という分析上の閾値であって離散的な遺伝子頻度ではない。
- 2 seed × 10,000tick は配線と観測のパイロットであり、自然選択成立の統計的証明には不十分。

## 課題
- 世代・系譜別の生涯繁殖成功率と親子相関の定量化：検証待ち
- 同一seedで植物毒タイプ頻度・毒圧を制御した比較：未着手
- 生存・繁殖への選択差の推定と複数seedによる検証：未着手
