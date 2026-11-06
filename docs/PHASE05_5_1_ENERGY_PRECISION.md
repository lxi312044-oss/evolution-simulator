# Phase05.5.1 長期エネルギー収支の精度改善

## 背景
5シードが約44k–45k Tickで `energy_delta ≈ -0.0173` により停止。

## 変更
- `ResourceLedger` の太陽光入力と熱放出の累積値に Neumaier 補償付き加算を採用。
- `ResourceBank::total_energy()` の合算に補償付き加算を採用。
- 保存則エラー時に台帳内訳と許容誤差を出力。
- 許容誤差の変更なし。生態系パラメータの変更なし。

## 注意
補償付き加算は集計誤差を軽減するが、口座間の個別加減算による丸め誤差や実装バグは解消しない。50,000 Tickの完走は未確認。

## 検証
`cargo test --workspace`、5シード×50,000 Tickのバッチ実行、`per_seed.csv` と `summary.csv` の確認。
