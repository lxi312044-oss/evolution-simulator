# Phase 06.2.3 — 捕食者と被食者の資源連動調整

## 実装
- `food_limited_reproduction_enabled`: 肉食動物の繁殖条件を全て満たしたときだけ、餌資源依存の確率で繁殖を許可。既定値 `false`。
- 繁殖成功確率 `P = H / (H + C * prey_per_predator_target)`。Hは現在の草食動物数、Cは現在の肉食動物数。既定 target=20。餌が減るほど出生が自然に減少する。
- `prey_refuge_enabled`: 草食動物が `prey_refuge_threshold` 以下のとき攻撃を禁止。既定値 `false`。植物保護に相当する任意の保護策。
- `repro_food_blocked` CSV列: 資源・年齢・クールダウンなど既存条件を全て満たした後、確率的繁殖制御で不成立となった延べ回数。
- 既存設定は追加項目省略時に従来挙動（refuge OFF、food-limited OFF）。

## 比較
`config/phase0622-carnivores.json`（従来）、`config/phase0623-natural.json`（自然調整）、`config/phase0623-refuge.json`（自然調整＋草食動物保護）。

## 検証
```powershell
cargo test --workspace
New-Item -ItemType Directory -Force results
cargo run --release -p sim-core --example carnivore_verification -- config/phase0623-natural.json 50000 100 results/carnivore0623-natural.csv
cargo run --release -p sim-core --example carnivore_verification -- config/phase0623-refuge.json 50000 100 results/carnivore0623-refuge.csv
```

**注意**: 生存は保証されない。単一シードの結果だけで安定共存と判定しない。30シード以上の長期比較を推奨。捕食イベント数は捕食死数と同義ではない。
