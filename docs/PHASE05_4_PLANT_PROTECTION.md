# フェーズ05.4 植物絶滅リスクの暫定的な捕食制限

## 背景
フェーズ05.3の単一シード比較では植物絶滅が満腹連動繁殖OFFでTick 4,600、ONでTick 5,000。長期共存は未達成。

## 仕様
- 設定 `herbivores.plant_protection_enabled`：初期値 false、UIでON/OFF切り替え。
- 設定 `herbivores.plant_protection_threshold`：初期値300、整数0～5000。
- Tickの草食動物更新開始時点で `living_plants <= threshold` なら全草食動物の捕食・植物へのターゲット探索を抑止。移動・代謝・寿命・繁殖判定は継続。
- 植物数が閾値を超えれば次のTickで捕食再開。境界付近ではON/OFFが繰り返され得る。
- 既存の満腹度ON/OFFと繁殖ON/OFFは独立。
- CSV累積カウンター `herbivore_plant_protection_blocked_ticks`。単位は草食動物×Tick。
- 捕食のN/E移転を抑制するだけで保存則を変更しない。

## 限界と次の検証
全域の個体数だけで判定する簡易的な人工保護策。植物の繁殖可能性や局所的な分布を保証しない。ON/OFFで同じシード・設定を用いて7000 Tickまで比較し、絶滅時期・植物数・草食動物数・出生・捕食・N/E誤差を確認する。Rust/WASM/ブラウザ検証は未実施。
