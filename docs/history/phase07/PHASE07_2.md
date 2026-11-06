# Phase 07.2

## Phase 07.2 — 遺伝性耐性と維持コスト（試験実装）

Phase 07.1.2 の D（低閾値）を毒作用の実験条件として採用。草食動物の `toxin_resistance` を遺伝させ、突然変異で変化させる。耐性による毒HPダメージ軽減と、エネルギーを熱へ散逸する維持コストを導入。毒OFFまたは耐性OFFでは旧動作を保つ。植物の毒生産コストは従来の保存則対応処理を継続する。

実験条件は `off`（毒なし）、`toxin_no_resistance`（毒あり・耐性なし）、`toxin_resistance`（毒あり・耐性あり）の3条件。すべて genetic・同じ5シード・各50,000 Tickで比較する。

```powershell
cargo test --workspace
.\run_phase072.ps1 -Ticks 1000 -SeedCount 1 -Force
.\run_phase072.ps1 -Force
```

**制約:** 植物毒性は現時点では固定で、植物の毒性遺伝は未実装。耐性の初期値は全個体共通であり、変異が十分に生じるまで選択の観測は難しい。耐性の平均値・分布・世代別変化の専用CSV集計とWeb個体選択UIは次の診断課題。既存の固定・待ち伏せモードは保持する。
