# Phase 04.3 診断CSV

- 0 Tickと100 Tickごとにスナップショットを記録。+500/+1000でも内部で100 Tick単位に分割して取りこぼしを防止。
- `type=snapshot` は累積値と `delta_*`（前回スナップショットとの差）を含みます。`type=error` は実行エラーを記録します。
- 植物死亡: `plant_age_deaths`, `plant_consumed_deaths`。草食動物死亡: `herbivore_age_deaths`, `herbivore_hp_deaths`（HP消耗は餓死由来が主ですが、原因断定を避けてHP死亡と表記）。
- 植物成長: `plant_growth_n`, `plant_uptake_n`, `plant_photosynthesis_e`, `mineralized_n`。
- 捕食: `plant_consumed_n`, `herbivore_absorbed_n`, `herbivore_feeding_e`, `herbivore_feeding_events`。
- 探索: `herbivore_target_acquisitions`, `herbivore_exploration_steps`, `herbivore_exploration_distance`, `herbivore_target_distance`, `herbivore_movement_e`, `herbivore_basal_e`。
- 保存則: `nutrient_error = total_n - initial_n`, `energy_error = total_e - initial_e - sunlight_in + heat_out`。
- CSVはUTF-8 BOM付き。設定適用・リセット時にログを初期化。CSVはその時点の記録を出力。
- 毎Tickの個体別イベント全件は保存せず、発生時にカウンタを更新して100 Tickごとに差分を出力。ブラウザのメモリ消費を抑制。
- 既存の生態系パラメータ・資源移動ロジックは変更しません。
