# Phase05.3 初期化修正

- 捕食停止・再開の数値入力を step=any に変更。Firefoxなどで min と step の刻み不整合が生じないようにする。
- 初期化時に停止0.8、再開0.4を明示設定。
- 範囲エラーには実際に読み取った文字列を表示。
- 既存の資源保存ロジックは変更なし。

確認: cargo test --workspace; wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg; cd web; npm run dev。
