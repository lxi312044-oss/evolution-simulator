# Phase 06.1 — 個体別体色・遺伝・突然変異・パレット変更

- 草食動物の `HerbivoreGenes.body_color` は `[0,1]` の sRGB RGB 値。
- 初期体色は seed と entity index から決定論的に生成。既存の生態系乱数列を消費しない。
- 無性繁殖で親のRGBを継承し、既存の mutation_probability / mutation_scale を用いて各チャンネルを独立に加算変異。体色変異は別の決定論的な乱数列で評価。
- 既存の `herbivore_snapshot_json` は4列のまま保持。新API `herbivore_color_snapshot_json` は `[x,y,radius,hpRatio,R,G,B]`。
- `web/src/sprites.ts` は既存16×16パターンの毛色パレットのみを変更し、目・輪郭・鼻などを維持。色別Canvasキャッシュは512件を上限にクリア。
- 色は見た目と遺伝子の共通データ。OKLab色認識、経験学習、配偶者選択、肉食動物の色遺伝は後続フェーズ。
- 色はN/E資源勘定に関与しない。既存の生態計算用RNG列を変更しない。

## 検証

```powershell
cargo test --workspace
wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg
cd web
npm install
npm run build
npm run dev
```

注：既存の長期実験の再現性は環境上で再実行して確認すること。
