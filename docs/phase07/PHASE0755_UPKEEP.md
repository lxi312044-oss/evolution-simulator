# Phase 07.5.5 — 複数毒の耐性維持コスト検証・調整

## 目的
複数毒での耐性維持コストの重複を検証し、毒HPダメージ・植物毒生産コストを変えずに追加タイプの維持コストだけを比較する。

## 計算式と変更範囲
タイプ0の従来の `resistance_upkeep` は変更しない。追加タイプ (1以上) のコストのみ `resistance_energy_cost_per_tick * sum(additional_type_resistances) * additional_type_resistance_upkeep_multiplier` とする。実際の支払いは保有エネルギーで上限を設け、従来どおり熱へ計上する。

`additional_type_resistance_upkeep_multiplier` は0〜1、既定1。**既定値ではPhase 07.5.2と完全に同じ**。0は追加タイプの維持コストのみゼロ、0.5は半額、1は従来どおり。毒タイプ別負荷・HPダメージ・植物の毒生産は変えない。設定値0は検証用で、恒久的な推奨値ではない。

## 実験
`tools/verify_phase0755.ps1` で全Rustテストと固定10,000Tick回帰。`tools/run_phase0755.ps1` で2seed×6条件=12run。CSVは `results/phase0755` に出力。条件: legacy_single, mixed_single_burden, mixed_multi_burden (倍率1), mixed_multi_upkeep_half (倍率0.5), mixed_multi_upkeep_zero (倍率0), mixed_no_toxin。既存の診断列と同一形式。

## 合格条件
1. 既定倍率1で従来の計算と回帰が一致。
2. 単一毒の動作は倍率を変えても不変。
3. 追加タイプの維持コストは倍率に対して非減少、会計は熱として保存。
4. 比較CSVを生成し、個体数、飢餓HP、毒HP、耐性維持エネルギーをseed別に比較。

## 判定上の注意
コスト減少によって絶滅率が改善しても自然選択成立の証明ではない。2seedは探索的比較で統計的確証に不十分。`mixed_no_toxin` は植物毒生産コストも消えるため、コストだけの因果比較には用いない。倍率0/0.5/1の3条件が主比較。絶滅・生存はseed依存の可能性がある。複数毒での総エネルギー消費は生存個体数・時間によって変化するため、累積費用だけで因果を断定しない。

## 状態
実装済み、Windowsでのコンパイル・実行・結果分析待ち。READMEのフェーズ管理では「検証待ち」。
