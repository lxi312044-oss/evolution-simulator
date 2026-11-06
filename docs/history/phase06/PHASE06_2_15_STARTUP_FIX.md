# フェーズ06.2.15 Web初期化修正

## 修正内容
- 起動時の設定は画面のフォーム値から生成せず、`web/src/default-config.json` をそのまま WASM に渡します。バッチ用 `config/phase06214-validation.json` と同じ設定を維持します。
- Firefox等でフォームが空欄に復元された場合、手動の「設定を適用してリセット」時には現在の検証JSONの値で補完します。
- 設定を変更する際は入力欄に有効な値を指定してください。範囲外の値は従来どおりエラーになります。

## Web実行
```powershell
wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg
cd web
npm install
npm run dev
```

画面をCtrl+F5で再読み込みしてください。Rust/WASMのビルド検証はこの環境では実施できていません。
