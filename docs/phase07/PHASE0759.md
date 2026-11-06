# Phase 07.5.9 — 耐性防御効果の介入比較と世代進化の検証

## 最終目的
毒タイプA/Bそれぞれの遺伝的耐性が、曝露と生存・繁殖の差を通して世代間で選択されるかを実証する。個体群の存続のみでは合格にしない。

## 変更範囲
既存の `resistance_effectiveness` 設定を利用し、同一seed・初期毒タイプ分布で防御あり(0.8)と防御なし(0.0)を比較する。**維持費は両群とも0.5倍で固定**。毒生産・繁殖・突然変異の式は変更しない。観測用CSVは07.5.8の既存例を再利用。

## 実験
A優勢(80:20)、B優勢(20:80)、均等(50:50) × seed12345/12346 × 防御あり/なし = 12ラン。各10,000Tick、100Tick間隔。

## 出力
`results/phase0759/seed*_*.types.csv`、`.births.csv`、`.outcomes.csv`、`.individuals.csv`、`paired_summary.csv`。

## 合格条件
1. 全Rustテストと旧版10,000Tick回帰がPASS。
2. 12ランとCSVが生成され、絶滅時の耐性平均を欠測扱いする。
3. 毒A/Bそれぞれの曝露・耐性・繁殖・世代間変化を評価する。
4. 防御あり/なしの比較で選択圧の寄与を検討する。ただし2seedの生態系全体への介入は個体レベルの因果効果を同定せず、遺伝的浮動・個体群変動も残る。進化実証は別途判断する。

## 課題管理
- 対応中: 防御有無による集団レベル差の評価
- 検証待ち: 12ラン、旧版回帰、毒タイプ別世代推移
- 継続観察: 植物の毒あり/なしと草食動物耐性の共進化（周期を前提にしない）

## 2026-11-06 解析品質の改善

`analyze_phase0759.py` は12条件の正確な組合せ、設定JSON、介入以外の条件一致、初期状態一致、seed、毒タイプ、Tickの全サンプル、有限値、個体数収支、出生死亡の窓別・累積整合を検査します。途中終了や欠損時は終了コード2、`quality.json` の `passed=false` を返します。以前の集計が残ることがあるため、必ず今回の終了コードと品質結果を確認してください。

従来の `paired_summary.csv` に加え、`paired_effects.csv` に同一seed・環境・毒タイプの防御ON−OFF差を出します。絶滅時の耐性とそれを含む差分は欠測。絶滅時刻は最終生存サンプルと最初の絶滅サンプルの間であり、正確な死亡Tickではありません。個体数と出生死亡は毒タイプ間で共通なので、タイプ行を合算しないでください。

```powershell
python -m unittest discover -s tools -p test_phase0759.py
python tools/analyze_phase0759.py results/phase0759 --ticks 10000 --interval 100 --out results/phase0759/audit-20261106
powershell.exe -ExecutionPolicy Bypass -File tools/run_phase0759.ps1 -Ticks 1000 -Interval 100 -OutputDirectory results/phase0759-smoke-20261106
```

出力先を指定すれば既存の長期実験を上書きせず短期動作確認できます。品質PASSはtypes CSVと設定の整合確認に限定され、個体履歴・親子系譜の完全性、実行コードの来歴、進化実証を保証しません。詳細と未解決事項: [検証報告](VALIDATION_20261106.md)。
