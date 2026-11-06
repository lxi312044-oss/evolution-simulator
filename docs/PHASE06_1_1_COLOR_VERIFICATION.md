# Phase06.1.1 — 体色遺伝の検証ログ

Phase06.1 の RGB 遺伝・突然変異・ドット絵パレットを変更せず、観測機能を追加しました。

## 実行（プロジェクトルート）

```powershell
cargo test --workspace
cargo run --release -p sim-core --example color_verification -- config/phase055-longrun.json 50000 100 12345 results/color0611 true
```

最終引数 `true` は出生ごとのイベントCSVを出力します。`false` ではイベントを記録せず、集計CSVのみ出力します。長時間・多シード検証は `false` 推奨です。

- `color_statistics.csv`: 100 Tickごと（指定間隔）の生存個体のRGB平均・母分散、体色継承回数、体色変異出生回数、体色変異チャンネル数、既存の出生・遺伝子変異回数、総N/E残高。
- `color_mutation_events.csv`: 出生1件ごとの親/子ID（index, generation）、親/子RGB、R/G/Bそれぞれの突然変異判定。ファイル名は従来の提案に合わせていますが、**変異のない出生も含みます**。変異だけを見る場合は `mutated_r + mutated_g + mutated_b > 0` で抽出します。

## 集計の解釈

- `color_inheritances` は成功した出生数と一致します。初期個体は含みません。
- `color_mutated_births` は少なくとも1つの色チャンネルで変異抽選に当選した出生数。
- `color_mutated_channels` はR/G/Bの当選回数の合計（1出生につき0〜3）。
- 色成分が上下限でクランプされ、変異判定に当選しても結果が親と同じになる可能性があります。
- `herbivore_mutations` はHP/速度/視野の従来の変異回数であり、体色変異回数とは別です。
- `total_n` / `total_e` は総残高であり、保存則の誤差そのものではありません。保存則の厳密な判定には既存の診断・World::stepの検証を併用してください。

## 設計上の注意

ログのために乱数を追加消費しません。色変異の抽選結果を既存の色生成処理から返すことで、変異回数を正確に数えます。イベントは明示的に有効化した場合だけ蓄積し、CSV出力のたびにdrainします。イベント出力を無効化しても集計カウンターは更新されます。

検証観点：同じseedでCSV一致、true/falseで集計CSV一致、`color_inheritances == herbivore_births`、イベントの行数が出生回数に一致、変異フラグ合計と集計値が一致、N/E保存則と従来の長期生存率に退行がないこと。
