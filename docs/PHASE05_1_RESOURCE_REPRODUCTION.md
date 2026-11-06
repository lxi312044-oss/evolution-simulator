# フェーズ05.1 資源連動繁殖（ON/OFF）

- `resource_dependent_reproduction`: false（既定・従来動作） / true（資源連動）
- `reproduction_food_radius`: 100（親の周囲を調べる半径）
- `reproduction_food_target`: 8（確率が100%となる植物数）

既存の繁殖年齢・HP・エネルギー・クールダウン条件を満たす親に対し、ON時のみ周囲の生存植物数 `k` を空間グリッドで数える。確率 `min(1, k / target)` に従って繁殖候補を選ぶ。候補になった後は既存の親→子N/E移転と繁殖作業熱を実行する。植物0体なら繁殖しない。OFF時は従来どおり（余分な乱数消費なし）。

CSVの `herbivore_reproduction_food_blocked` は累積の資源連動確率判定での却下回数。`delta_` 列は直近100 Tick分。植物の「個体数」を局所資源の代理指標として使用しており、植物の実N量ではない。捕食量・摂食量の制御は変更しない。設定適用はリセット後に反映される。

## 検証

1. `cargo test --workspace`。
2. 循環改善プリセットで資源連動OFF、ONをそれぞれ7,000 Tick実行し、CSVを別名保存する。
3. 草食動物・植物の個体数、出生、HP死、 `herbivore_reproduction_food_blocked`、N/E保存誤差を比較する。
4. 同じシードで複数の設定を比較し、長期安定性は別途検証する。
