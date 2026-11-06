# フェーズ06.2.19 Web検証

Web画面の上部で5条件を切り替えられます。条件変更時はリセットされ、バッチと同じ設定JSONを読み込みます。

```powershell
cargo test --workspace
wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg
cd web
npm install
npm run build
npm run dev
```

`http://localhost:5173` を開き、同じSeedでバッチと比較してください。CSVは100 Tickごとに記録されます。

**注意:** Web画面で設定を手動編集して「設定を適用」した場合は、バッチと同じ条件ではなくなります。草食動物HP死亡時のEゼロは餓死と断定できません。
