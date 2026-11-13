# Webのプレイ版と開発版

リポジトリのルートでWASMを生成し、Webサーバーを起動する。

```powershell
wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg --no-opt -- --features state-audit
node tools/test_web_inspector.mjs
cd web
npm run build
npm run dev -- --host 127.0.0.1 --port 5175 --strictPort
```

- `/play` はマップ・再生・速度・個体数・個体選択。ホイールまたは拡大ボタンでズームし、拡大中はドラッグで移動。個体が重なった場合は個体情報欄の候補を選ぶ。円で選択中の個体を示す。
- `/dev` は同じマップに詳細設定と観測を追加。設定の変更後は「適用して新規開始」。適用済み設定JSONは環境識別子と観測設定を含み、従来の単体設定JSONも読み込める。対照の保存は画面内の設定と集団数の控え。同一seedで再開始してログを比較できる。草食系譜記録には終了Tickを指定する。終了前の系譜出力には未完了フラグが付く。
- 両画面で `?preset=stage4-fixed-candidate` または `?preset=stage4-fixed-baseline` を指定できる。URLで指定しない場合は「無毒の緑から始まる世界」で起動する。従来の通常毒は `?preset=standard-toxin`。研究設定と成果物の出典は [段階4の固定仕様](STAGE4_FIXED_EXPERIMENT.md)。

設定編集にはRustが受け付けるスカラー項目を使う。自由な初期配置と記憶の時間忘却は未実装。色距離・記憶容量・空腹による許容式や動物毒の移送率はコードの規則。個体欄は実際の保存フィールドを表示し、未保存の出生Tickや親ID、植物の独立した毒在庫、個体ごとの直近行動理由や繁殖判定の全ゲートは「未取得」と表示する。現在のEによる許容値は表示時点の値で、過去の判定結果を表さない。

[Issue #8](https://github.com/lxi312044-oss/evolution-simulator/issues/8) に検証結果と残項目を記録する。

## Webの初期プリセット

「無毒の緑から始まる世界」（`green-founders`）は通常毒から次の値だけを上書きする。

| 設定キー | 値 |
| --- | --- |
| `toxin.enabled` | ON |
| `toxin.plant_variation_enabled` | ON |
| `toxin.initial_toxic_plant_fraction` | 0 |
| `toxin.plant_toxic_mutation_probability` | 0.05 |
| `plants.color_inheritance_enabled` | ON |
| `plants.color_mutation_probability` | RGB各成分につき0.20 |
| `plants.color_mutation_scale` | 0.25 |
| `herbivores.color_learning_enabled` | ON |
| `carnivores.speed` | 4.2 距離/Tick（通常毒は3.2） |
| `toxin.experimental_plant_toxicity_colors`、`toxin.experimental_inherited_plant_colors` | OFF |

色と毒性の値は観察用。URL指定と開発版で読み込んだJSONはそれぞれの設定で開始する。リセットは適用中の設定で再開始する。CLIとエンジンの既定値は従来どおり。

## 初期全無毒・緑を手動で設定する

開発版の「通常毒」を選び、「植物・色」の実験配色2種をOFFにする。「植物の色の継承・変異」と「植物の有毒・無毒形質の継承と変異」をONにし、初期・土壌発芽時の有毒率を0にする。観察例では色変異確率0.20、最大幅0.25、有毒・無毒の反転確率0.05を使用できる。毒A/Bも変異させる場合は毒型の継承をON、毒型数を2とし、毒型変異確率を別途設定する。

「毒・耐性」で植物毒の作用をONにし、「学習・判断」で草食の植物色学習をONにする。「適用して新規開始」で反映し、適用中の設定JSONを出力できる。例の数値は観察用。既定は色継承OFF、確率と最大幅は0。

種子出生時に、RGB各成分について独立に変異を判定する。変異成分には正規化RGBの最大幅の範囲で一様に増減を加え、0〜1に収め、最も近い8ビット値に丸める。丸めや範囲制限で見た目が変わらない場合もある。変異幅0または確率0なら親の色をそのまま継承する。世界seedと子の完全IDを使う植物色専用の乱数系列で、毒性反転・毒型変異の抽選と生態計算の乱数から分離する。初期株と親なしの土壌発芽は従来の緑で、生存中は色を更新しない。草食は実際に経験した被毒と植物の可視色を結び付ける。毒と色の対応は固定されない。
