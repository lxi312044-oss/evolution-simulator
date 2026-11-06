# Phase 07.5.2 草食動物の毒タイプ別負荷と耐性遺伝子

## 目的
植物の毒タイプ遺伝子と草食動物の独立した毒負荷・耐性遺伝子を接続する。旧来のタイプ0の計算経路を保持する。

## 変更範囲
- `toxin.herbivore_multi_toxin_enabled` を追加。既定値は `false`。
- 草食個体に `typed_load: [f64;16]`、草食遺伝子に `typed_resistance: [f32;16]` を追加。
- タイプ0は旧来の `toxin_load` / `toxin_resistance` を引き続き使用。
- 有効時は植物の `toxin_type` に応じて負荷を加算。タイプ1以降は独立して除去・閾値判定・耐性軽減。
- タイプ1以降の耐性は親から遺伝し、独立した決定的ハッシュストリームで突然変異。既存RNGの消費順は変更しない。
- 有効時のみタイプ1以降の耐性維持コストを追加し、熱収支へ計上。

## 設定例
```json
{
  "plant_toxin_type_variation_enabled": true,
  "plant_toxin_type_count": 2,
  "herbivore_multi_toxin_enabled": true,
  "resistance_enabled": true
}
```
上記キーは `toxin` オブジェクト内に配置する。

## 合格条件
1. 毒タイプ1の負荷はタイプ0の負荷に混ざらない。
2. タイプ0の耐性はタイプ1のダメージを軽減しない。
3. 追加の耐性遺伝子が親子で保持され、突然変異が決定的である。
4. 既定の無効状態で旧版10,000Tick回帰検証が合格する。
5. 全Rustテストと資源保存検証が合格する。

## 未実装・制限
- タイプ別毒量・耐性頻度の世代別CSVと選択差・親子相関の集計。
- 毒タイプごとに異なる生理パラメータの生態系設定（現時点では同じ除去率・閾値・ダメージ係数を利用）。
- 動物による毒生成・肉食動物への毒伝播。
- Web UIのタイプ別可視化。

## 検証
`powershell.exe -ExecutionPolicy Bypass -File ".\tools\verify_phase0752.ps1"`

## 状態
検証待ち。開発環境にRust toolchainがないため、Rustテストと旧版回帰比較はWindowsで実施する。
