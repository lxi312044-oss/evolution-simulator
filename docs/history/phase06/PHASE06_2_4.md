# Phase 06.2.4: 肉食動物のエネルギー収支と捕食成功率

## 目的
06.2.3では草食動物が残存しても肉食動物がエネルギー不足で絶滅した。エネルギーの消費内訳、獲得量、遭遇状況を切り分けて検証する。

## 追加した累積診断列
- `hunt_attempts`: クールダウン・保護を通過した攻撃回数
- `hunt_positive_energy`: 実際にエネルギーを得た攻撃回数
- `hunt_zero_energy`: エネルギーを得られなかった攻撃回数
- `energy_basal_spent`: 基礎代謝で消費したE
- `energy_movement_spent`: 移動で消費したE
- `energy_reproduction_spent`: 繁殖作業で消費したE
- `predator_target_ticks`: 視界内に獲物がいた肉食動物Tickの延べ回数
- `predator_search_ticks`: 視界内に獲物がいなかった肉食動物Tickの延べ回数
- `mean_carnivore_energy`: 生存肉食動物の予備E平均（瞬間値）
- `mean_carnivore_reserve_n`: 生存肉食動物の予備N平均（瞬間値）

捕食成功率は `hunt_positive_energy / hunt_attempts` で算出する。これは「エネルギーを得た攻撃率」であり、「草食動物を殺した率」ではない。既存の `carnivore_hunts` は攻撃イベント数。

## 比較用設定
- `phase0624-diagnostics.json`: 06.2.3自然調整と同じ条件（基準）
- `phase0624-low-cost.json`: 基礎代謝0.018、移動コスト0.004（消費軽減）
- `phase0624-high-encounter.json`: 視野160、攻撃範囲9（遭遇改善）
- `phase0624-refuge.json`: 低コスト+草食動物保護（参考）

**調整は設定値のみ。** 個体数の強制補充やエネルギーの無償生成は行わない。

## 実行
```powershell
cargo test --workspace
New-Item -ItemType Directory -Force results
foreach ($mode in @('diagnostics','low-cost','high-encounter','refuge')) {
  cargo run --release -p sim-core --example carnivore_verification -- "config/phase0624-$mode.json" 50000 100 "results/carnivore0624-$mode.csv"
}
```

## 注意
Rustツールチェーンがない環境で作成したため、コンパイルと長期試験は未実施。各CSVの保存誤差、出生・死亡、共存を確認してからパラメータを採用する。複数シード検証は次段階。
