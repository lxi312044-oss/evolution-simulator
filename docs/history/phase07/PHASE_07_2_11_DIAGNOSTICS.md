# Phase 07.2.11 — 選択圧のばらつきの原因分析（探索的）

- 既存＋追加ペアシード: 20。F−B平均生存率差: -1.294 pt。
- 新規コホート詳細があるシードペア: 15。

## 1シード除外感度（平均差の変化が大きい上位）

| Seed | F−B (pt) | 除外後平均 (pt) | 平均変化 (pt) |
|---|---:|---:|---:|
| 12364 | -19.323 | -0.345 | +0.949 |
| 12355 | +12.315 | -2.010 | -0.716 |
| 12351 | -12.807 | -0.688 | +0.606 |
| 12357 | -11.562 | -0.753 | +0.540 |
| 12346 | +8.878 | -1.829 | -0.535 |

## 新規15シードの探索的相関

相関はシード単位の記述統計であり、原因を確定しません。特に曝露量と維持コストは生存期間に依存します。

| 指標 | Pearson r |
|---|---:|
| F_toxin_dose_diff | +0.957 |
| F_toxin_damage_diff | +0.938 |
| F_prevented_damage_diff | +0.876 |
| F_resistance_cost_diff | +0.758 |
| B_largest_cohort_share | -0.365 |
| F_largest_cohort_share | -0.354 |
| F_weight | +0.349 |
| F_cohorts | +0.296 |
| B_resistance_cost_diff | +0.243 |
| B_weight | -0.172 |
| B_cohorts | -0.150 |

## 判定

- Phase 07.2.11の記述的診断は完了。原因の因果的同定は未達。
- 元CSVの再送・Rust再実行は不要。未観測の捕食圧・初期耐性分布は今回の表から推測しない。
- 次段階で個体群規模・初期遺伝子分布・捕食圧のシード別要約をローカル抽出し、共有するのが望ましい。

## データ制約

- Original 5 seeds have only paired summaries; no original cohort-level covariates in this input
- Cumulative dose/cost and cohort sizes may be consequences of survival; correlations are non-causal
- No individual population size, predation pressure, or initial allele distribution in supplied aggregate tables
- Exploratory multiple correlations; no significance testing or causal interpretation
