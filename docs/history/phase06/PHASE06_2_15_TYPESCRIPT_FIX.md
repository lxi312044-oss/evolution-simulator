# Phase 06.2.15 TypeScript ビルド修正

- `web/src/default-config.json` と `config/phase06214-validation.json` に Rust の `HerbivoreSettings` に存在する `reproduction_satiety_threshold` (0.5)、`reproduction_well_fed_ticks` (100) を追加。Webとバッチで同じ設定に統一。
- `web/src/main.ts` のCSVメトリクス作成時、WASMから返る `bigint` の攻撃・出生・死亡累計を `Number(...)` に変換。
- Rustのロジック・探索アルゴリズムは変更していません。

## 検証手順
```powershell
cargo test --workspace
wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg
cd web
npm install
npm run build
npm run dev
```

注: wasm-packによる `web/src/pkg` の生成は利用者環境で実施してください。
