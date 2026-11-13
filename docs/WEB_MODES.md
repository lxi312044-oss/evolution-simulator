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
- 両画面で `?preset=stage4-fixed-candidate` または `?preset=stage4-fixed-baseline` を指定できる。既定の起動は既存標準設定。研究設定と成果物の出典は [段階4の固定仕様](STAGE4_FIXED_EXPERIMENT.md)。

設定編集にはRustが受け付けるスカラー項目を使う。自由な初期配置や植物RGBの独立変異率、記憶の時間忘却は未実装。色距離・記憶容量・空腹による許容式や動物毒の移送率はコードの規則。個体欄は実際の保存フィールドを表示し、未保存の出生Tickや親ID、植物の独立した毒在庫、個体ごとの直近行動理由や繁殖判定の全ゲートは「未取得」と表示する。現在のEによる許容値は表示時点の値で、過去の判定結果を表さない。

[Issue #8](https://github.com/lxi312044-oss/evolution-simulator/issues/8) に検証結果と残項目を記録する。
