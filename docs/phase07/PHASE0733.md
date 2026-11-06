# Phase 07.3.3 — 生物種共通の毒データ構造

## 目的
既存の単一毒シミュレーションを変えず、毒の識別子・性質・体内蓄積・遺伝耐性・生産能力を独立した型で表現する。

## 仕様固定
- `ToxinTypeId(u16)`: `LEGACY=0`。ID 1以降の毒は現段階で処理しない。
- `ToxinProfile`: 毒の曝露・解毒・損傷のパラメータ。旧 `ToxinSettings` から精度を落とさず構築。
- `ToxinBurden`: 毒IDと体内毒量（f64）。
- `ToxinResistance`: 毒IDと耐性値（f32）。毒保有や生産と独立。
- `ToxinProduction`: 毒IDと生成量・生成コスト・有効フラグ。現段階では未使用。
- 草食動物の旧フィールドを正本とし、`Herbivore::toxin_burden()` と `toxin_resistance()` が値コピーの互換ビューを返す。既存フィールドは増やさない。

## 非対象
植物/肉食動物への状態フィールド追加、毒生産の個体差、複数毒の同時作用、捕食伝播、色学習、擬態。これらは後続フェーズで実装。

## 検証
1. `powershell.exe -ExecutionPolicy Bypass -File ".\tools\verify_phase0733.ps1"`
2. `wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg`
3. `cd web; npm install; npm run build; npm run dev`
4. 旧プリセットの毒OFFと通常毒ONを確認。個体構造体のフィールドは不変であることをコードレビュー。

比較は基準CSVの行順に依存せず、全列・重複件数を厳密に検証する。Rust/WebテストはWindowsでの実施待ち。
