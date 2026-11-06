# Phase 06.2.5 — 草食動物の増減傾向に連動した肉食動物の繁殖制御

## 変更
- 既存の個体数比による繁殖判定に加えて、直近500 Tickの草食動物の個体数減少率および出生・死亡数の差を観測します。
- 草食動物が減少しているときのみ、肉食動物の繁殖成功確率を最大85%低下させます。完全禁止ではありません。
- 草食動物保護は既存の `prey_refuge_enabled` で切り替え可能。自然調整ではOFF、比較用保護条件のみON。
- 機能OFFでは追加の乱数を消費しません。資源N/Eの移動処理は変更しません。
- CSVに `repro_trend_blocked`, `prey_decline_fraction`, `prey_births_window`, `prey_deaths_window` を追加。

## 比較
- `phase0625-baseline.json`：06.2.4基準と同等（新機能OFF）
- `phase0625-natural.json`：増減傾向制御ON・保護OFF
- `phase0625-refuge.json`：増減傾向制御ON・保護ON

```powershell
cargo test --workspace
New-Item -ItemType Directory -Force results
foreach ($mode in @('baseline','natural','refuge')) {
  cargo run --release -p sim-core --example carnivore_verification -- "config/phase0625-$mode.json" 50000 100 "results/carnivore0625-$mode.csv"
}
```

## 検証上の注意
- `repro_trend_blocked` は延べ繁殖判定数です。
- 減少率は前回観測時点と現在の個体数から計算し、直近観測窓で更新されます。
- 50,000 Tickの単一シードで共存が確認できても安定共存とは断定できません。最低30シードで確認してください。
- これまでのログから3種共存を保証できる根拠はありません。保護ONでも肉食動物は絶滅し得ます。
