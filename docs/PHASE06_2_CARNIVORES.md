# Phase 06.2 — 肉食動物（実装・検証）

## 機能

- `carnivores.enabled` は既定 `false`。OFF時は旧シミュレーションを維持。
- ONかつ `initial_population.carnivores > 0` の場合、初期肉食動物を生成。
- 肉食動物は最寄りの草食動物を視野内で追跡し、射程内で草食動物の栄養N・エネルギーEを自分の資源口座に移転する。
- 移動・基礎代謝でエネルギーを熱として散逸する。寿命・エネルギー枯渇で死亡し、Nを腐植へ移す。
- 資源が十分なら無性繁殖。親から子へ資源を移し、体色RGBを遺伝・突然変異させる。
- 肉食動物の描画には既存のキツネ型16×16ドット絵を利用。体色は個体別RGB。
- 性的繁殖・配偶者の呼びかけ・OKLab学習・色選好は **未実装**（Phase06.3以降）。

## 検証ログ

```powershell
cargo test --workspace
cargo run --release -p sim-core --example carnivore_verification -- config/phase062-carnivores.json 50000 100 results/carnivore062.csv
```

`results` ディレクトリは先に作成してください。出力項目は `tick, plants, herbivores, carnivores, carnivore_hunts, carnivore_births, carnivore_deaths, nutrient_error, energy_error`。

```powershell
wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg
cd web
npm install
npm run build
npm run dev
```

Webの既定設定はOFF。Web表示でONにするには `web/src/default-config.json` の `carnivores.enabled` を `true`、`initial_population.carnivores` を例えば `12` に変更して再ビルドしてください。設定画面のON/OFFコントロールは今後追加予定です。

## 制約・未検証

この成果物はコンテナにRust/Cargoがないためコンパイルと長時間検証を実行できていません。必ず上記のコマンドで確認してください。複数シード評価は次の検証ステップです。草食動物のHPが0になった後の死体処理は既存の草食動物更新処理に従います。
