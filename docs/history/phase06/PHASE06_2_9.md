# Phase 06.2.9 — 胃容量・消化・繁殖制限の比較

## 目的
Phase06.2.8の「仕留めた瞬間に予備資源へ大量移転＋固定250Tick休息」に代えて、仕留めた獲物からの追加資源を胃の独立したN/E口座へ移し、徐々に予備資源へ移す。

- `digestion_enabled` はデフォルトOFF。旧設定と旧動作を維持。
- 胃の容量はEとN別々に制限。満腹度は両者の最大比率で計算。
- 満腹度80%以上で狩猟・移動停止、25%以下で再開（ヒステリシス）。基礎代謝は継続。
- 胃が25%を超える間は繁殖を保留。胃からの資源移動は保存的。
- 旧方式の通常攻撃（bite）は従来どおり直接予備資源へ移す。胃は致死攻撃時の追加摂食に適用。
- 死亡時には胃の残余Nを腐植へ、Eを熱へ移す。子の胃は空で初期化。
- 草食動物保護は標準OFF。絶滅防止の強制補充は実装しない。

## 検証
```powershell
cargo test --workspace
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\run_phase0629.ps1 -Ticks 1000 -SeedCount 2
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\run_phase0629.ps1
```

5条件（baseline, timer, digestion, digestion-slow, digestion-refuge）、各10シード×50,000 Tick。
`results/phase0629/` にCSV出力。N/E保存則、3種生存、出生数、胃摂食/消化累計、満腹による狩猟・繁殖阻止を比較する。

**注意:** Rustビルド・実行はこの作業環境で未確認。共存の改善は未判定。
