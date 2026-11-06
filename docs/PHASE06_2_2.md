# Phase06.2.2 — 繁殖資源の整合性と捕食圧の調整

- **繁殖修正:** 捕食で蓄えた reserve N から子の body N と reserve N を両方支払う。親の body N を新たに作り直すことはしない。N/E移動は既存台帳を使用。
- **捕食クールダウン:** `attack_cooldown_ticks` を導入。接触ごとの連続攻撃を制限する。
- **低密度保護（実験的）:** `prey_refuge_threshold` 以下では捕食攻撃を行わない。絶滅防止の強制ルールになるため、自然選択を優先する評価では **0に設定** して比較すること。デフォルトは0（無効）で、検証用の介入策としてのみ有効化可能。
- **調整プリセット:** `config/phase0622-carnivores.json` は肉食動物初期4匹・攻撃間隔20Tick。従来の`phase062-carnivores.json`も使用できる。
- **診断ログ:** `hunt_refuge_blocked`, `hunt_cooldown_blocked` をCSVに追加。既存の繁殖失敗・保存誤差も維持。`repro_body_n_blocked`は歴史的な列名で、現在は繁殖に必要なreserve N合計不足を意味する。

## 検証

```powershell
cargo test --workspace
cargo run --release -p sim-core --example carnivore_verification -- config/phase0622-carnivores.json 50000 100 results/carnivore0622.csv
```

初回は保存誤差、出生数、草食動物・肉食動物の生存期間を確認する。複数シードでの検証は未実施。
