# フェーズ04.4 生態系診断強化

既存の100 Tick CSVログに以下の診断を追加。動植物の資源移動・成長・移動パラメータは変更しない。

## 追加カウンター（累積および100 Tick差分）
- `seed_attempts`: 成熟かつクールダウン解除後の散布確率試行回数
- `seed_probability_rejected`: 確率抽選による失敗
- `seed_immature`, `seed_cooldown_blocked`: 未成熟・クールダウンで試行前に除外した回数（個体×Tick）
- `seed_resource_blocked`: 確率抽選後に親の資源不足で失敗した回数
- `seed_capacity_blocked`: 最大個体数到達で散布ループを中断した回数
- `plant_growth_nutrient_limited`, `plant_growth_energy_limited`, `plant_growth_zero`: 成長可能な植物の制約回数（個体×Tick）。栄養とエネルギーの両方が不足すれば重複計上する
- `herbivore_search_without_target`, `herbivore_target_ticks`, `herbivore_zero_energy_ticks`: 動物×Tickでの探索・追跡・エネルギー欠乏

## 追加スナップショット（100 Tick時点の状態）
- 土壌セルNの最小・最大、ゼロセル数、半飽和定数未満のセル数
- 植物の分布セル数・最大セル密度・成熟/未成熟・寿命まで100 Tick以下の個体数・平均成長度
- 植物の体N・蓄積N・蓄積E、草食動物の体N・蓄積N

## 注意
- `seed_immature` 等は死亡原因ではなく散布阻害の延べ回数。
- 植物の捕食死は体Nがほぼ0になったときに記録。部分摂食は `plant_consumed_n` で確認。
- `soil_below_half_saturation_cells` は土壌セル単位であり、個々の植物が栄養不足で死亡したことを意味しない。
- ログの出力ファイルは `ecosystem-phase044-tickXXXX.csv`。
- 比較実験では同一シード・同一パラメータを用いる。
