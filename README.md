> **運用変更（2026-11-06、ユーザー承認）**：課題・現在状態・実験計画・実施結果・判断・次の対応の正本は [GitHub Issues](https://github.com/lxi312044-oss/evolution-simulator/issues) です。[運用台帳 #4](https://github.com/lxi312044-oss/evolution-simulator/issues/4) と [Phase 07.5.9の結果 #26](https://github.com/lxi312044-oss/evolution-simulator/issues/26) を参照してください。READMEは概要・導入・設計参照を担い、以下の課題表・進捗・旧運用記述は移行前の履歴です。旧記述がREADMEへの進捗記録を指示していても、今後はIssuesを更新します。科学的な合格条件・既存証拠・履歴は維持します。詳細: [GitHub運用](docs/GITHUB_WORKFLOW.md)。

> **旧Phase 07.5統合検証・Phase 08移行ゲート（履歴）**：旧条件と判定は [PHASE075_GATE.md](docs/phase07/PHASE075_GATE.md) に保持します。今後の開発順序・受入は下記の5段階ロードマップを適用します。既存のゲートスクリプトの出力を、新方針の自動判定とは扱いません。

> **Phase 07.4.6（統合設計案・レビュー待ち）**：草食・肉食の毒生成/耐性/蓄積/食物連鎖への伝播を設計。**この版は動物毒を実装しません。** 正式ロードマップでは動物毒の本実装はPhase 08です。設計: [PHASE0746.md](docs/phase07/PHASE0746.md)。

# 生態シミュレーション — 現在の開発方針とフェーズ移行基準

> **現行ロードマップ（2026-11-11、ユーザー指定）**：[毒・色・攻撃の5段階ロードマップ](docs/DEVELOPMENT_ROADMAP.md)。段階実装と機能検証を行い、全体を接続して調整した後、設定を固定して研究検証します。過去の研究条件・証拠・未達判定は保持します。

## テーマ：毒・色・攻撃を通じて、動物の生存戦略が変化する生態系

動物は相手の色と経験から、攻撃・摂食・回避を選びます。毒は危険を与え、毒種別の耐性はその危険を軽減します。植物は食物網と毒経験を支える基盤とし、主役は動物です。適応進化・擬態は重要な研究目標として維持し、その未達だけで次の機能開発を停止しません。

## 現行の5段階ロードマップ

| 順序 | 開発・検証 | 終了の基準 |
|---|---|---|
| 1 | 現在の毒・耐性の基盤を受け入れる | 独立作用、遺伝、保存則、容量、再現性、互換・安定性・性能を対象版で確認。適応進化の成立は待たない |
| 2 | 色学習と攻撃・摂食・回避の判断を実装する | 植物→草食の既存経路から、経験による判断変化とON/OFFを機能検証 |
| 3 | 動物毒と食物連鎖への伝播を実装する | 詳細設計を確定し、動物同士の毒・色・攻撃を接続して統合検証 |
| 4 | 全体を動かして設計値を調整する | 少数の因子ごとに変更理由と結果を残し、採用設定を固定 |
| 5 | 設定を固定し、進化・擬態を研究検証する | 調整標本と確認標本を分け、固定計画の結果を成立・不確実・不成立にかかわらず記録 |

各段階の詳細と旧フェーズの対応は [開発ロードマップ](docs/DEVELOPMENT_ROADMAP.md) を参照。現在状態・受入証拠はIssuesを正本とします。段階1の入口は [#17](https://github.com/lxi312044-oss/evolution-simulator/issues/17)、色学習は [#19](https://github.com/lxi312044-oss/evolution-simulator/issues/19) です。今回の方針採用は、受入PASSや次段階の実装完了を意味しません。

## 研究目標：毒・色・学習からベイツ型擬態が自然発生する生態系

植物・草食動物・肉食動物のそれぞれに有毒・無毒の個体が存在する。毒の生成には資源コストがあり、毒を持つことと特定毒への耐性は独立して遺伝する。捕食者は摂食・被毒経験から相手の色を学習して危険な色を避ける。その結果、無毒の被食者のうち有毒個体に似た色の系統が自然選択で増え、毒生成コストを払わない擬態が自発的に生まれることを目指す。**擬態の色への変化や回避結果を直接ハードコードしない。** 色は連続的に遺伝・変異し、学習した記憶自体は遺伝させない。擬態個体が増えすぎると回避効果が弱まる頻度依存性も長期目標とする。

## 現在の基準と注意

- **通常毒（1倍）** は旧実験名「毒10倍」に相当し、HP損傷係数は `0.010`。旧`x10`ログは再現性のため改名しない。
- 現在の標準実験/Webは単一毒で全植物が毒を持つ条件。これは**耐性選択の実験環境**であり、最終的な有毒・無毒混在生態系ではない。
- `SimulationConfig::default()`は後方互換性のため毒OFF。`config/standard-toxin.json`とWeb起動時の通常毒は別途設定される。
- Phase 07.2.28で毒耐性進化の観察的な証拠を得たが、**自然選択の因果的な証明は未達**。継続課題として維持する。

## 旧ロードマップの目的・成果物・進行判定（履歴）

以下は採用前の計画を保全したものです。今後の着手順序・依存関係には上記5段階を適用し、研究成立を要求する旧条件は研究目標として保持します。特に植物擬態の実証を動物毒実装の必須前提から外します。過去の未達を達成へ読み替えません。

| フェーズ | 目的 | 作成・確認するもの | 次へ進む条件 |
|---|---|---|---|
| **07.3.1 現状調査・仕様固定** | 既存の単一毒の挙動を変えずに共通化計画を確定 | 毒処理の所在と計算順、SHA-256基準、静的監査、回帰比較仕様、課題管理 | 静的監査合格、`cargo test --workspace`合格、Web通常毒の確認、比較用旧実装ログ保存 |
| **07.3.2 生理作用の共通化** | 排出・損傷・耐性軽減の計算を生物種非依存の関数に抽出 | 共通モジュール、単体テスト、草食からの呼出 | 同seed・同設定・同Tickで旧版と計算/診断が一致、既存テスト全合格、資源保存 |
| **07.3.3 共通毒データ構造** | 毒量・耐性・毒種を植物/草食/肉食に拡張可能な型へ整理 | 共通データ型、既存草食への互換アダプタ、未有効化の動物拡張口 | 旧設定/ログの互換性、毒OFF/ONの回帰一致、不要な個体メモリ増加を評価 |
| **07.3.4 共通化の長期回帰検証** | 共通化が生態系と選択圧を壊さないと証明 | 複数seedの長期比較、毒・繁殖・資源診断、Web検証 | 旧版との同条件比較に合格、例外/資源保存異常0、差分の理由を文書化 |
| **07.4 有毒・無毒混在と毒生成コスト** | 毒の有無・生成量を個体別の遺伝形質にする | 有毒/無毒植物、生成費、遺伝・変異、混在実験 | 有毒/無毒が共存可能、生成費が資源収支へ反映、遺伝と変異が再現 |
| **07.5 複数毒と種別耐性** | 複数毒への異なる耐性とコストの進化を可能にする | 複数毒、種類別蓄積・解毒・損傷・耐性、診断 | 毒A/Bの作用と耐性が独立、単一毒互換、長期安定性と性能を確認 |
| **07.6 色の学習・捕食回避** | 捕食者が経験から色の危険度を学習 | 個体別記憶、色類似度、遅延した毒経験、空腹時の判断、ON/OFF | 学習ONで危険色回避が増え、OFFでは基準挙動、似た色へ一般化し誤認も起こる |
| **07.7 植物の擬態の創発検証** | 無毒植物の色が有毒植物へ自然に近づくか検証 | 多seedの系譜・色・生存・繁殖ログ、学習OFF対照 | 無毒の類似色個体が生存・繁殖で有利、世代を通じた収束、学習OFFで優位が弱まることを確認。未成立なら次へ進まず原因分析 |
| **08 動物の毒と擬態** | 草食/肉食の毒生成・蓄積・捕食伝播・擬態を実装 | 動物毒、種間毒移動、色学習と遺伝、動物の擬態実験 | 毒・エネルギー収支が正しく、動物でも学習依存の擬態を再現 |
| **09 共進化** | 毒・色・耐性・捕食者の識別能力が相互に進化するか検証 | 学習特性の遺伝、頻度依存、長期複数seed実験 | 形質の変化が自然選択・系譜・対照実験で説明でき、安定性と再現性を確認 |

各作業は**目的・変更範囲・合格条件を着手時に固定**します。開発受入と研究判定を分け、技術的前提が未達なら関連する開発を保留し、科学的な未達は研究Issueへ残します。課題状態は「未着手・対応中・検証待ち・解決済み・先送り・継続観察」。解決済みには根拠、先送りには理由と再検討条件を残し、自然現象とバグを区別します。

## Phase 07.3.1 の確認コマンド

```powershell
python tools/audit_phase0731.py
cargo test --workspace
wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg
cd web
npm install
npm run dev
```

上記のうち本環境で実行済みなのはPython静的監査のみ。Rustのビルド、テスト、Web動作はWindowsでの確認が必要。**現時点の07.3.1判定は「検証待ち」**。

---

## 標準毒（Phase 07.2.28以降）とWeb表示

- 旧実験名「毒10倍」は、今後 **通常毒（1倍）** と呼びます。HP損傷係数は **0.010** です。過去の `x10` 結果ファイル名は履歴・再現性のため変更しません。
- 標準実験設定: `config/standard-toxin.json`（毒ON、遺伝的耐性ON）。従来の `config/phase0722-F_strong_cost.json` の係数0.001に10を掛けた条件と同じです。
- Web起動時の初期条件を標準毒に変更し、平均耐性・高耐性個体数・自然発芽累計を表示します。
- Webではバッチと同じ自然発芽パラメータ（閾値20、確率0.005、20Tick間隔、最大20）を有効にします。自然発芽はRustの既存機能をWASMから呼び出します。
- Web起動方法（Windows PowerShell）: プロジェクトルートで `wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg`、次に `cd web; npm install; npm run dev`。表示されたローカルURLにアクセスしてください。
- 注意: Web表示は現在の**ライブ実行**であり、過去140ランの履歴を再生するものではありません。過去の実験と比較するには同じseed、Tick、設定を使用してください。UIの「設定JSONを出力」には自然発芽の実行時設定が含まれないため、バッチにそのまま渡すだけでは完全再現できません。
- 既存Rust `SimulationConfig::default()` は後方互換性のため毒OFFのままです。「通常毒」はWebの起動設定と標準実験JSONの意味です。

# 初期コンセプト（原案・将来構想）

> この節はプロジェクトの出発点となった構想を記録するものです。**現段階の実装仕様や開発優先順位を拘束しません。** 未実装の機能も含みます。

生態シュミレーションを作りたい  
植物、草食動物、肉食動物これらを生物と呼称する

植物は土地の肥沃度が高いと発生する  
草食動物は植物を食べ繫殖し増える  
肉食動物は草食動物を食べ繫殖し増える

生物は色を持っている  
生物はHPを持っている  
生物はHPが0になると死体になる  
死体を生物は食べることができる  
生物は毒を持つことがある  
毒は食べた相手のHPを減らす  
毒は色々な種類がある  
毒は生物が持つ抗体で防ぐことができる  
抗体は特定の毒しか防げない

植物は動くことができない  
植物は成長する  
植物は成長するとHPの上限が増える  
植物の成長は止まる  
植物の成長が止まると種をまくことができる  
種からは自分と同種の植物が育つ

草食動物、肉食動物のことを動物と呼称する  
動物は動くことができる  
動物は攻撃力を持つ  
動物は生物を攻撃しHPを減らすことができる  
動物は成長する  
動物は成長するとHPの上限と攻撃力が上がる  
動物は色で相手に攻撃をするかしないかを決める

テーマは毒と色と攻撃の生態シュミレーションです  
色で相手から逃げるか相手を追うかを選択し生き残りをシュミレーションする  
植物はあくまで動物の生態シュミレーションの土台であり主役は動物です

---

## Phase06.2.5

[増減傾向による繁殖制御・比較手順](docs/PHASE06_2_5.md) を参照。

# 生態進化シミュレーション v1.2 — Phase 04

## 詳細設計書

本プロジェクトの設計方針、データモデル、主要アルゴリズム、処理順序、テスト方針は、以下の詳細設計書を参照してください。

**[生態シミュレーション 統合実装設計書 v1.0](docs/DETAILED_DESIGN.md)**

設計書には、植物・草食動物・肉食動物、遺伝・突然変異、RGB/OKLabによる色認識、学習・記憶、毒・抗体、戦闘、Utility AI、資源・エネルギー、1 Tickの処理順序、実装フェーズを記載しています。

> **設計と実装の区別**：設計書は全体構想を含む実装基準版です。現行READMEのPhase 04の実装状況と、設計書に記載された将来機能は一致しない場合があります。実装済み機能の確認には本READMEを参照してください。

GitHub等で共有する際は、`README.md` と `docs/DETAILED_DESIGN.md` を同じリポジトリに配置してください。README上のリンクから設計書を閲覧できます。


フェーズ03を基に草食動物80体を追加しました。植物300体、草食動物80体、肉食動物0体（フェーズ06予定）。

## 起動（PowerShell）

```powershell
rustup target add wasm32-unknown-unknown
cargo test --workspace
wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg
cd web
npm install
npm run dev
```

## 追加仕様
- 草食動物の初期遺伝子：HP45〜75、速度1〜4、視界35〜110（シード付き乱数）。
- 植物探索：Uniform Grid、セル幅50、最寄り植物を視界内から選択。
- 移動：速度遺伝子と化学エネルギー残量で制限。植物がなければランダム探索。
- 捕食：半径5以内で植物の身体栄養を取得。吸収できない栄養は有機物へ移す。
- 食事エネルギー：植物の既存化学エネルギーから移転、余剰は熱へ散逸。
- 代謝・飢餓：エネルギーが0なら最大HPの1%/Tick損傷。死亡時に資源を有機物へ返す。
- 世界総栄養Nは保存。初期草食動物の資源も初期総量に計上。
- 動物の繁殖・戦闘・毒・学習は後続フェーズ。

## 注意
この環境ではcargo/rustcが見つからないため、Rustのコンパイルとブラウザ起動は未検証です。既存のPhase03をバックアップしてから展開してください。


## Phase 04 movement fix (2026-10-31)

`movement: InsufficientBalance` was caused by rounding the affordable travel distance to `f32` and later recomputing an `f64` debit that could exceed the energy balance. The movement code now debits `min(desired_cost, available_energy)` **before** changing coordinates and derives the actual travel distance from that exact charge. Movement errors include tick, animal ID, account ID, energy balance, charge, cost coefficient, and distance. Regression tests cover near-zero energy and 3000 ticks.

Run `cargo test --workspace`, rebuild WASM with `wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg`, and restart Vite. Rust compilation has not been run in the generation environment.


## フェーズ04.1 生態系バランス調整

画面下部の「生態系バランス調整」から、植物成長速度、土壌吸収上限、種子散布確率、草食動物の摂食上限、初期草食動物数を変更できます。

- 「旧設定」: フェーズ04の従来値（比較用）。
- 「回復重視」: 成長0.009、吸収0.3、散布確率0.65、摂食0.35、草食動物80（実験用の仮設定）。
- 変更後は「設定を適用してリセット」を押すと、Tick 0から新規Worldを作成します。
- 設定変更は実行中のWorldへ反映しません。旧Worldを保持したまま新Worldの初期化に成功した場合だけ切り替えます。
- 同じseedで複数の設定を比較してください。生態系の長期安定は未検証です。

注意: 本段階では種子バンクは未実装で、植物が0体になると自然再生しません。


## フェーズ04.4 診断強化

100 Tick間隔のCSVログに、植物の種子散布失敗要因・成長制約・土壌栄養のセル分布・生物の資源内訳・草食動物の探索状態を追加しました。仕様は [docs/PHASE04_4_DIAGNOSTICS.md](docs/PHASE04_4_DIAGNOSTICS.md) を参照してください。

## フェーズ04.5（資源循環・繁殖条件の比較）

- 土壌Nの隣接セル間拡散率、有機物無機化率、種子散布半径、クールダウンをUIで調整可能。
- 「循環改善（試験設定）」プリセットを追加。旧設定・回復重視も維持。
- 100 TickごとのCSVに `soil_diffused_n` / `delta_soil_diffused_n` を記録。
- [設計・検証手順](docs/PHASE04_5_CIRCULATION.md)。**実機検証待ち**。

## フェーズ05（実装・検証待ち）
草食動物の単為生殖、遺伝子継承、突然変異、資源保存型の出生処理を追加。詳細は `docs/PHASE05_REPRODUCTION.md`。100 Tick CSVで `herbivore_births` などを追跡できます。

### フェーズ05.1：資源連動繁殖 ON/OFF
草食動物の繁殖を「従来方式（OFF）」または「周辺植物数に応じた確率（ON）」に切り替えられます。ブラウザ設定画面で半径と基準植物数を変更し、「設定を適用してリセット」で反映します。詳しくは `docs/PHASE05_1_RESOURCE_REPRODUCTION.md`。Rust/ブラウザ実行は未検証です。

## フェーズ05.2 満腹度（実装・検証待ち）

満腹度ON/OFF、胃N口座、消化、捕食停止・再開（ヒステリシス）、CSV診断を追加。詳細は [`docs/PHASE05_2_SATIETY.md`](docs/PHASE05_2_SATIETY.md) を参照。資源連動繁殖は独立して設定可能です。

## Phase05.3 満腹連動繁殖（検証待ち）
満腹度ONのとき、一定の満腹度を連続維持した草食動物だけ繁殖可能にする設定を追加。独立ON/OFF、詳細は `docs/PHASE05_3_SATIETY_REPRODUCTION.md`。

## フェーズ05.4：植物絶滅リスクへの暫定対策（実装済み・動作検証待ち）

**未解決の重要課題：植物の絶滅。** フェーズ05.3の比較試験では満腹連動繁殖OFFでTick 4,600、ONでTick 5,000に植物が絶滅した。繁殖抑制だけでは捕食圧を十分に下げられず、植物と草食動物の長期共存は未達成。

暫定対策として「植物保護の捕食制限」を追加した。ON時は世界全体の生存植物数が設定閾値以下なら、草食動物は植物を探索対象とせず捕食しない（既定閾値300体）。閾値を超えれば次Tickから再開する。OFFでは従来方式。既定はOFF（比較用）。この対策は全域共通の人工的な保護ルールであり、自然な生態学的機構ではない。植物が寿命や資源不足で絶滅する可能性は残る。

CSVには `herbivore_plant_protection_blocked_ticks`（捕食保護のために捕食不可となった草食動物×Tickの累積）を追加。N/Eの移転処理は変更しない。`docs/PHASE05_4_PLANT_PROTECTION.md` と `docs/VERIFICATION.md` を参照。


### Phase05.4 初期化エラー対策
Firefox等で設定入力値が復元され、満腹継続Tickに `0.8` が入ると初期化が失敗する問題に対し、起動時に `default-config.json` から関連設定を復元する処理を追加しました。再ビルド後にブラウザを再読み込みしてください。

## フェーズ05.5：複数シード・長期実行・統計評価

Rustネイティブのバッチ評価ツールを追加しました。詳細は [docs/PHASE05_5_BATCH_EVALUATION.md](docs/PHASE05_5_BATCH_EVALUATION.md) を参照してください。

```powershell
cargo run --release -p sim-core --example batch_evaluation -- config/phase055-longrun.json 50000 100 12345,12346,12347,12348,12349 results/phase055
```

**継続課題：植物の絶滅**。フェーズ05.4の植物保護・満腹度・満腹連動繁殖ONでは20,200 Tickまで共存を観測しましたが、複数シードでの長期安定性は未確認です。Tick 45,145でエネルギー保存則エラー（energy_delta ≈ -0.01735）が報告されており、原因の切り分けが必要です。バッチ評価はこのエラーを握りつぶさず、シード別の失敗として記録します。

**検証状況：実装済み、コンパイル・実行未検証。** 現段階ではブラウザUIへの統合はありません。


## Phase05.5.1 — 長期エネルギー収支の数値精度（実装・実行検証待ち）

5シード（12345–12349）すべてが44,179～45,243 Tickでエネルギー収支エラー（約 -0.0172～-0.0174 E）により停止した。植物・草食動物の絶滅ではなく、保存則チェックで停止。

- 累積太陽光入力・熱放出に補償付き加算を適用。
- エネルギー口座合計にも補償付き加算を適用。
- エラー時に現在E、初期E、太陽光累積、熱放出累積、許容誤差を表示。
- 保存則の許容誤差（絶対1e-3、相対1e-9）は変更しない。
- **未検証**：50,000 Tick完走。各口座の個別更新による丸め誤差は残り得るため、同じエラーが再発した場合は別途原因調査が必要。

```powershell
cargo test --workspace
cargo run --release -p sim-core --example batch_evaluation -- config/phase055-longrun.json 50000 100 12345,12346,12347,12348,12349 results/phase0551
```

## フェーズ06.1 — 動物のドット絵表示

草食動物をウサギ型の16×16ドット絵でCanvas描画し、肉食動物用のキツネ型ドット絵を用意しました。肉食動物はまだ実装されていないため、現段階では凡例にのみプレビュー表示されます。シミュレーションの数値計算・保存則・CSV形式は変更していません。詳細は [`docs/PHASE06_1_SPRITES.md`](docs/PHASE06_1_SPRITES.md) を参照してください。


## Phase 06.1 体色遺伝・スプライト再配色
草食動物の個体別RGB体色・無性繁殖での継承と変異・16×16ドット絵の再配色を実装。詳細は [docs/PHASE06_1_COLOR_GENETICS.md](docs/PHASE06_1_COLOR_GENETICS.md)。肉食動物・OKLab色認識・色選好は未実装。

## Phase06.1.1 体色遺伝の検証ログ

`docs/PHASE06_1_1_COLOR_VERIFICATION.md` を参照。集計CSVと任意の出生イベントCSVを追加しました。既存の色遺伝と生態系の計算処理は維持します。

## Phase 06.2 肉食動物

肉食動物の探索・追跡・捕食・代謝・寿命・無性繁殖・体色遺伝と検証CSVを追加。既定OFF。詳細は [docs/PHASE06_2_CARNIVORES.md](docs/PHASE06_2_CARNIVORES.md)。

### Phase 06.2.1 — extinction diagnostics and plant pixel art

See [`docs/PHASE06_2_1_DIAGNOSTICS.md`](docs/PHASE06_2_1_DIAGNOSTICS.md) for the expanded predator CSV diagnostics, their interpretation, and 16×16 growth-stage plant sprites. Plant toxicity and color evolution remain future work.


## Phase06.2.2

繁殖資源・捕食圧調整と検証方法は [docs/PHASE06_2_2.md](docs/PHASE06_2_2.md) を参照。

## Phase 06.2.3 — 餌資源連動型の肉食動物繁殖制御

実装内容、設定・検証手順は [Phase 06.2.3 詳細](docs/PHASE0623.md) を参照してください。


## Phase 06.2.4 — Carnivore energy diagnostics
See [PHASE06_2_4.md](PHASE06_2_4.md) for metrics, comparison configurations and execution steps.


## Phase 06.2.6 — 草食動物保護の30シード比較

保護OFF / 閾値20・50・100を同じ30シードで比較します。**標準設定は変更していません。**

```powershell
cargo test --workspace
.\run_phase0626.ps1
```

詳細: [docs/PHASE06_2_6.md](docs/PHASE06_2_6.md)

## Phase 06.2.7 — Predator hunting and energy-flow investigation

This phase is **diagnostic-only**: it does not change predator movement, hunting, reproduction, metabolism, refuge, RNG calls, or resource transfers. It adds cumulative counters to `CarnivoreDebug` and appends nine columns to the `carnivore_verification` CSV:

- `prey_visible_out_of_range`: predator-ticks with visible prey but no prey in attack range after movement.
- `prey_in_attack_range`: predator-ticks with prey in range (including cooldown/refuge-blocked opportunities).
- `predator_movement_distance`: accumulated distance traveled.
- `attack_energy_limited`: attacks that acquired less than configured bite energy (including prey reserves being exhausted).
- `prey_hp_zero_after_attack`: hits that brought a prey's HP to zero; this is **not** necessarily the same as deaths processed later.
- `predator_death_energy_remaining`: energy in predator reserve at death.
- `predator_death_age_total`: total age at death; divide by carnivore_deaths for average.
- `predator_death_nearest_prey_distance_total` and `predator_death_nearest_prey_samples`: nearest prey distance at death, only when prey exists.

Use the original phase 06.2.5 natural-balance configuration, so the comparison is not confounded by parameter changes. `run_phase0627.ps1` generates per-seed CSVs and `results/phase0627/summary.csv` for 30 seeds. Each output CSV retains the old 39 columns before the new ones. Run:

```powershell
cargo test --workspace
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\run_phase0627.ps1 -Ticks 1000 -SeedCount 2
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\run_phase0627.ps1
```

**Important:** This is an instrumentation release, not a claim of improved coexistence. Compare the time series of energy gained vs basal/movement/reproduction costs, attack opportunities vs attacks, and prey distance at predator death. The supplied environment did not have a Rust toolchain, so compile and runtime tests have not been performed here.

## Phase 06.2.8
See [PHASE06_2_8.md](PHASE06_2_8.md) for post-kill satiety and five-mode 10-seed evaluation.

## Phase 06.2.9
胃容量・段階的消化・満腹度に応じた狩猟/繁殖制限を追加（既定OFF）。詳細は [PHASE06_2_9.md](PHASE06_2_9.md)。検証: `run_phase0629.ps1`。


## Phase 06.2.10
See [PHASE06_2_10.md](PHASE06_2_10.md) for unified feeding and 10-seed evaluation.


## Phase 06.2.11
See [PHASE_06_2_11.md](PHASE_06_2_11.md) for adaptive digestion tests and diagnostic columns.

## Phase 06.2.12

Energy-aware carnivore hunting experiment: see [`docs/phase06-2-12.md`](docs/phase06-2-12.md). Run `run_phase06212.ps1` (4 conditions × 10 seeds by default).

## Phase 06.2.13 — Persistent carnivore exploration
See [docs/phase06-2-13.md](docs/phase06-2-13.md). Run `run_phase06213.ps1` (4 modes × 10 seeds).

## Phase 06.2.14

[肉食動物のWeb探索改善と再ビルド手順](docs/phase06-2-14.md)

## フェーズ06.2.14：5シード再検証

肉食動物の草食動物型探索移動を検証するため、`run_phase06214.ps1` を追加しました。既定では5シード（12345〜12349）、各50,000 Tickで実行し、`results/phase06214/` にCSVと集計結果を出力します。手順は `PHASE06_2_14_VALIDATION.md` を参照してください。


## Phase 06.2.15 — Web / Batch verification

See [PHASE06_2_15.md](PHASE06_2_15.md). Both environments use the phase06.2.14 validation JSON. Web now displays carnivore metrics and exports/imports active JSON. Batch defaults to five seeds.


## Phase 06.2.16 — Carnivore reproduction investigation
See [docs/phase06-2-16.md](docs/phase06-2-16.md). Run `run_phase06216.ps1` (5 seeds, 50,000 ticks, 3 conditions). No forced survival.


## Phase 06.2.17 — Carnivore reproduction diagnostics

See [PHASE06_2_17.md](PHASE06_2_17.md). Five seeds × 50,000 ticks × three cooldown variants. Includes separate resting, active search, chasing and reproduction-eligibility diagnostics.


## Phase 06.2.18 — Energy balance experiment

See [PHASE06_2_18.md](PHASE06_2_18.md). Run `run_phase06218.ps1` for five seeds and four energy-cost conditions.


## Phase 06.2.19
See [food chain supply validation](docs/phase06-2-19.md). Run `run_phase06219.ps1` for five conditions × five seeds × 50,000 ticks.


## フェーズ06.2.20

食物連鎖の周期停止を調査する診断フェーズです。詳細は [docs/phase06-2-20.md](docs/phase06-2-20.md) を参照してください。

## Phase 06.2.21 — Herbivore energy diagnostics
See `docs/phase06-2-21.md`. Run `run_phase06221.ps1` for batch comparisons.


## Phase 06.2.22
See [PHASE06_2_22.md](PHASE06_2_22.md) for matched-seed herbivore reproduction-gate experiments. Run `run_phase06222.ps1`.


## Phase 06.2.23
See [PHASE06_2_23.md](PHASE06_2_23.md) for the plant-recovery/herbivore-reproduction experiment.


## Phase 06.2.26
See [PHASE06226.md](PHASE06226.md) for the satiety-gate comparative test and run instructions.


## Phase 06.2.27
See [PHASE06227.md](PHASE06227.md) for the 100/80/60-tick satiety comparison and plant-resource balance analysis.


## フェーズ06.2.37 — 標準検証運用

1. **Rustテスト**: `cargo test --workspace` で変更の回帰を確認する。
2. **短時間のバッチ検証**: PowerShellで1,000 Tick程度を実行し、CSVの列・行数・終了Tickを確認する。
3. **正式な定量評価**: PowerShellの `run_phase06237.ps1` を使用し、同一条件で **50,000 Tick × 5シード（12345～12349）× 3条件** を実行する。必要に応じシード数を増やす。
4. **Web版**: 行動・描画・不自然な動きの観察に使う。気になる動作があった場合にCSVと状況を共有する。Web版だけで統計的優劣を判断しない。
5. **解析**: `analyze_phase06237.py` で生存・絶滅Tick・捕食・繁殖・遺伝子分布・保存則誤差を比較する。異常値があれば該当シードのログを調べる。
6. **再現性**: 実行した設定JSONとCSVを条件・シード別に保存し、設定を変更した場合は別の条件名で管理する。

詳細とコマンド: [PHASE06237.md](PHASE06237.md)

## Phase 06.2.38: safe batch and gene behavior validation

Use `run_phase06238.ps1` for multi-seed experiments (default 50,000 ticks x 5 seeds x 3 strategies), and the Web version for visual inspection and anomalous behavior reports. The batch runner skips matching complete CSVs, reruns incomplete/mismatched ones, and writes results atomically after successful validation. Use `-Force` for a deliberate full rerun. Run `python analyze_phase06238.py` for gene-bin births, deaths, parent reproduction, and ambush/search behavior ratios. See `PHASE06238.md`.


## Phase 06.2.39 — Predator starvation and prey-avoidance diagnosis

Run `./run_phase06239.ps1 -Ticks 1000 -SeedCount 1` for smoke testing, then `./run_phase06239.ps1` for 50,000 ticks × 5 seeds × 3 strategies. Use Web for visual anomaly reports; use the PowerShell batch as the quantitative source of truth. Share `results/phase06239` as ZIP. Completed runs are cached with metadata, source hash, and CSV validation; partial runs are regenerated.

This phase **does not change** pursuit, escape, feeding, or energy mechanics. It adds diagnostic columns for prey near but beyond predator vision, low-energy predators with visible prey, nearby prey while satiated, low-energy predators without visible prey, herbivore flight despite visible plants, and threats at the detection boundary. These counters are cumulative individual-ticks, not distinct individuals.

Genetic ambush mutations now occur with the carnivore `mutation_probability` and use `mutation_scale`, rather than mutating every birth. The `gene_mutation_trials` counter counts successful mutation draws, while `gene_mutations_bin*` counts observable value changes. This change affects genetic-mode trajectories; do not directly compare with older phase results as if the dynamics were identical.

Observed symptoms under investigation: herbivores sometimes form a ring around predators and jitter between food pursuit and escape; predators may starve while prey exist elsewhere in the world. Do not interpret the existence of nearby prey as proof of a reachable, visible, attackable, or digestible meal. Diagnose perception, energy, movement, attack cooldown, satiety and reproduction separately.


## Phase 06.2.40 — Predator death diagnostics

Run `run_phase06240.ps1 -Ticks 1000 -SeedCount 1` for a smoke test, then `run_phase06240.ps1` for the standard 50,000 ticks × 5 seeds × 3 strategies. `analyze_phase06240.py` reads the batch CSV files. Completed runs are cached by source/configuration/tick count and incomplete runs are replaced via `.partial` files.

Phase 06.2.40 **does not change animal behavior**: it records the predator's state at death. Added cumulative counters: `p40_death_prey_visible`, `p40_death_prey_attack_range`, `p40_death_prey_absent`, `p40_death_feeding_paused`, `p40_death_nearby_prey_twice_vision`, `p40_death_energy_starvation`. Visibility uses predator vision and physical distance at death; it does not imply the predator could actually attack. `p40_death_feeding_paused` means the feeding-active flag was false at death; it does not alone prove the stomach was full. Death counters may overlap and should not be summed. Existing `predator_death_stomach_energy` and distance counters remain available.

**Validation policy:** Rust tests and short PowerShell runs first, then full paired-seed batch evaluation. Web is used to observe unusual behaviors and share targeted logs. Do not change ecological parameters until diagnostics have been reviewed.


## Phase 06.2.41
捕食追跡プロセスの診断: `PHASE06_2_41.md` を参照。実行: `run_phase06241.ps1`、集計: `analyze_phase06241.py`。


## Phase 06.2.42
詳細は [PHASE_06_2_42.md](PHASE_06_2_42.md) を参照。個体別死亡・生存診断CSV、Web +100 Tickと×1/×2/×4再生を追加。

## 開発方針決定（2026-11-03）：Phase 06 → Phase 07

**決定**：Phase 06.2.43を最後の診断基盤整備フェーズとし、その後Phase 07「毒・抗体・色の進化」を段階的に開始する。標準の長期検証は **genetic（遺伝子型）のみ**、50,000 Tick × 5シード（12345～12349）とする。`fixed`・`ambush` の実装は削除せず、必要時の対照・回帰検証用に残す。

**判断理由**：06.2.42までに遺伝型で長期生存例（Seed 12346）、個体別エネルギー・死亡診断を得た。一方、追跡距離計測、捕食間隔、系譜の検証に課題がある。毒・抗体・色を同時に導入すると、死亡原因や自然選択の因果関係が不明瞭になる。絶滅自体は自然な生態現象となり得るため、**肉食動物の絶滅率ゼロは移行条件ではない**。生存を保証する人為的補正は行わない。

**実施順序**：

1. **06.2.43**：同一獲物のTick間相対距離、捕食間隔・エネルギー、親子IDによる系譜の診断。毒なし基準系を保存する。
2. **07.1**：植物の毒1種類の曝露・蓄積・代謝・生理作用。毒なし対照群と比較する。
3. **07.2**：遺伝性耐性、毒生産と耐性維持のコスト。
4. **07.3**：色と毒の独立遺伝、色に対する危険度学習、擬態。
5. **07.4**：毒4種類への拡張、食物連鎖を通じた影響。

**Phase 07移行判定**：同じSeedと設定でのバッチ・Web再現性、長期の栄養N・エネルギーE保存則、個体の収支と死亡理由の追跡可能性、既知の追跡距離計測の問題解消、毒なし対照群の再実行可能性を確認する。未達の項目があれば記録し、必要な診断だけを行う。絶滅を防ぐための恣意的な調整はしない。

**設計原則**：遺伝子は能力・行動傾向に影響し、経験・学習・環境との相互作用で結果が変わる。毒性・耐性・色には利益とコストを設け、常に最適な固定戦略を作らない。生態メカニズムは原則1つずつ導入して同一Seedで変更前後を比較する。

06.2.43の詳細は [PHASE_06_2_43.md](PHASE_06_2_43.md) を参照。

## Phase 06.2.43 → Phase 07.1: baseline and design decision (2026-11-03)

- Phase 06.2.43 genetic baseline is frozen in `baselines/phase06243/phase06243-results.zip` with SHA-256 in `manifest.json`.
- All five genetic seeds ran to 50,000 ticks. Seed 12346 retained seven carnivores; the other four ended with zero. Extinction is a valid ecological outcome, not an automatic test failure.
- **Lineage correction:** the previously reported ancestry cycle was a false alarm caused by interpreting an index without its generation. `verify_phase06243_lineage.py` validates full `(id_index,id_generation)` identifiers: no missing parent and no cycles in all five life CSV files. No lineage behavior change was needed.
- Phase 07 order: 07.1 one fixed plant toxin and physiological effects → 07.2 inherited resistance and maintenance cost → 07.3 color recognition and learning → 07.4 four toxin classes and trophic transfer.
- Phase 07.1 adds `toxin` config (`enabled` defaults to false), per-herbivore toxin burden, dose from actual plant nutrients eaten, fractional clearance, HP damage above a threshold, and plant energy synthesis cost dissipated through the existing heat ledger. **All plants have the same fixed toxin strength in 07.1; no genetic toxicity, immunity, color association, or predator transfer yet.** This is an intentionally controlled intervention, not an evolutionary conclusion.
- Use `config/phase071-toxin-off.json` and `config/phase071-toxin-on.json` with the **same seed** to compare. Default toxin-disabled behavior should reproduce the frozen baseline; this still requires running the Rust tests and batch verification.
- Phase 07.1 is a preliminary implementation pending compilation and experiment validation; do not treat this archive as a validated scientific result.

### Baseline lineage check

```powershell
python .\verify_phase06243_lineage.py .\baselines\phase06243\phase06243-results.zip

```

### Validation

```powershell
cargo test --workspace

```

### Phase 07.1: 毒専用テストと対照比較（検証手順）

- `cargo test --workspace`：`crates/sim-core/tests/toxin071.rs` に追加した毒なしの不変性、摂食曝露、排出と閾値ダメージ、植物の生産コストと保存則、設定検証を含む。
- `./run_phase071.ps1 -Ticks 1000 -SeedCount 1 -Force`：毒なし（off）と毒あり（on）の短時間ペア検証。
- `./run_phase071.ps1 -Force`：同一Seed（12345～12349）、各50,000 Tick、geneticで毒なし・毒ありを比較。
- `python ./analyze_phase071.py`：Phase 06.2.43の基準CSVと毒なしCSVの主要指標を全Tickで照合し、毒ありとの差分を集計。`results/phase071/comparison.json` を生成する。
- 短時間検証時は `python ./analyze_phase071.py --ticks 1000 --seed-count 1` を使う。ただし、50,000 Tickの基準CSVと比較するため、短時間検証では基準との差が出る場合がある。**正式な基準一致判定は50,000 Tickで実施する。**
- **重要：** 既存のCSVは毒量・毒による死亡を独立列として記録していない。今回の比較は個体数、死亡、保存則の差を評価する段階であり、毒固有の因果分析は今後の計測追加が必要。
- 検証前にRustコンパイル・実行テストを必ず実施する。Linux側にRust環境がない場合はWindows側で検証する。

### Phase 07.1.1 — 毒の作用診断（追加）

毒の挙動を変更せず、累積診断列を `carnivore_verification` CSV末尾へ追加する。
`toxin_exposure_events`（摂食による曝露回数）、`toxin_exposure_total`（累積投与量）、
`toxin_clearance_total`（累積排出量）、`toxin_hp_damage_total`（毒によるHP損失）、
`toxin_damage_ticks`（毒ダメージを受けた個体Tick）、
`toxin_plant_energy_cost_total`（植物の毒生産による累積エネルギー消費）、
`toxin_deaths_with_burden`（死亡時に毒量が残っていた個体数）、
`toxin_deaths_with_burden_and_hp_zero`（そのうちHPが0以下の個体数）。

**注意:** 死亡時に毒が残っていることは、毒が直接死因だったことを意味しない。
毒の直接死亡寄与を確定するには個体別のダメージ源履歴と反事実比較が必要。
既存CSVの列は変更せず末尾に追加するため、旧Phase 06.2.43の対照データとの比較は共通列で行う。

```powershell
cargo test --workspace
.\run_phase071.ps1 -Ticks 1000 -SeedCount 1 -Force
python .\analyze_phase0711.py --ticks 1000 --seed-count 1
.\run_phase071.ps1 -Force
python .\analyze_phase071.py
python .\analyze_phase0711.py
```


## Phase 07.1.2 — 毒の作用条件の感度分析

毒の作用がほとんど発現しない問題を調査するため、**一度に一つのパラメータのみ変更**する。標準は genetic、5シード（12345–12349）、各50,000 Tick。

| 条件 | 変更 | 目的 |
|---|---|---|
| A_baseline | 変更なし | Phase 07.1.1の基準 |
| B_high_dose | dose_per_nutrient 0.2→0.4 | 曝露量の感度 |
| C_slow_clearance | clearance_fraction_per_tick 0.03→0.015 | 蓄積・排出の感度 |
| D_low_threshold | damage_threshold 2.0→1.0 | ダメージ発現の感度 |
| E_zero_cost | plant_energy_cost_per_tick 0.002→0 | 植物側の負担を分離 |

`cargo test --workspace` の後、`./run_phase0712.ps1 -Ticks 1000 -SeedCount 1 -Force` で短期確認し、`./run_phase0712.ps1` で長期実験。`python ./analyze_phase0712.py` で集計する。結果は `results/phase0712/` に保存。各条件の設定JSONと完了メタデータを保存する。

**判断上の注意：** 毒保有時死亡は毒による死亡を意味しない。累積HP損失と植物生産コストは単位が異なり直接比較しない。5シードの差を因果効果の確定値として扱わず、同一シードの対応比較と保存則を確認してからPhase 07.2へ進む。

## Phase 07.2 — 遺伝性耐性と維持コスト（試験実装）

Phase 07.1.2 の D（低閾値）を毒作用の実験条件として採用。草食動物の `toxin_resistance` を遺伝させ、突然変異で変化させる。耐性による毒HPダメージ軽減と、エネルギーを熱へ散逸する維持コストを導入。毒OFFまたは耐性OFFでは旧動作を保つ。植物の毒生産コストは従来の保存則対応処理を継続する。

実験条件は `off`（毒なし）、`toxin_no_resistance`（毒あり・耐性なし）、`toxin_resistance`（毒あり・耐性あり）の3条件。すべて genetic・同じ5シード・各50,000 Tickで比較する。

```powershell
cargo test --workspace
.\run_phase072.ps1 -Ticks 1000 -SeedCount 1 -Force
.\run_phase072.ps1 -Force
```

**制約:** 植物毒性は現時点では固定で、植物の毒性遺伝は未実装。耐性の初期値は全個体共通であり、変異が十分に生じるまで選択の観測は難しい。耐性の平均値・分布・世代別変化の専用CSV集計とWeb個体選択UIは次の診断課題。既存の固定・待ち伏せモードは保持する。


## Phase 07.2.1 — 耐性遺伝子の進化診断

既存の行動・遺伝・毒パラメータは変更せず、観測専用の実行例 `resistance_evolution_0721` を追加。`run_phase0721.ps1` で genetic を含む既存3条件を同一シードで実行し、各100 Tickの耐性平均・分散・5階級分布・毒量・エネルギー・HP・累積耐性維持コスト・軽減ダメージを記録する。出生と死亡の個体ID（index + generation）および当時の耐性値をイベントCSVに出力する。

```powershell
cargo test --workspace
.\run_phase0721.ps1 -Ticks 1000 -SeedCount 1 -Force
python .\analyze_phase0721.py
.\run_phase0721.ps1 -Force
python .\analyze_phase0721.py
```

**制限:** 親子IDは現在の草食動物データ構造から取得できないため空欄。突然変異の個体別帰属はできず、累積件数のみ。死亡イベントのHP・エネルギー・毒量は死亡直前の前Tick観測値であり、死因の確定値ではない。累積軽減ダメージは耐性の効果を示すが、進化の証明には世代ごとの形質頻度と環境別の適応度比較が必要。個体ごとの維持コストや回避ダメージも今後の課題。4毒A/B/C/Dと各耐性はPhase 07.4で追加予定。


## Phase 07.2.2 — 自然選択の検証（目的・合格条件を固定）

### 目的
「高耐性へ進化させる」のではなく、**遺伝子の違いが環境依存の生存・繁殖差を生み、世代間の遺伝子分布に反映されるか**を検証する。予想と異なる結果を恣意的に調整しない。原因を診断し、生態学的に説明できる結果を尊重する。

### 合格条件
1. **遺伝性**：親から子への耐性値継承と突然変異が確認できる（07.2.1.1で確認）。
2. **適応度の差**：同じ出生時期の個体間で、耐性による生存・繁殖成功度の差が毒の有無・強度・維持コストと整合する。高耐性が常に有利である必要はない。
3. **世代間の変化**：複数シードで環境に応じた耐性分布の変化が観測される。平均値の単純な変動だけで合格にしない。中立対照を使い、遺伝的浮動・突然変異と区別する。

### 実験条件（genetic、各50,000 Tick、初期5シード）

| 条件 | 毒 | 耐性維持コスト/tick | ダメージ閾値 | 目的 |
|---|---|---:|---:|---|
| A_no_toxin_no_cost | OFF | 0 | 1.0 | 中立対照 |
| B_no_toxin_cost | OFF | 0.002 | 1.0 | 維持コストのみ |
| C_toxin_no_cost | ON | 0 | 1.0 | 毒の利益のみ |
| D_toxin_cost | ON | 0.002 | 1.0 | 利益とコスト |
| E_strong_no_cost | ON | 0 | 0.5 | 強毒下の利益 |
| F_strong_cost | ON | 0.002 | 0.5 | 強毒下の利益とコスト |

**強毒条件では閾値のみ変更**。毒の濃度・排出速度・植物毒生産コストは変更しない。すべての条件で `resistance_enabled=true` とし、毒OFF条件でも遺伝子継承と維持コストを検証できるようにする。固定・待ち伏せモードは変更しない。

### 検証コマンド（PowerShell）

```powershell
cargo test --workspace
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
.\run_phase0722.ps1 -Ticks 1000 -SeedCount 1 -Force
python .\analyze_phase0722.py
.\run_phase0722.ps1 -Force
python .\analyze_phase0722.py
```

`analyze_phase0722.py` は `summary.csv`、`cohort_resistance.csv`、`pre_extinction.csv` を生成。出生1000 Tick幅のコホートを低/中/高耐性に区分し、観測終了まで1000 Tickの追跡期間がある個体のみ比較。子の数は**追跡期間内に実際に生まれた子**を数える。生存率は追跡期間末時点で評価する。絶滅時の直前10サンプルを保存する。個体数が少ない区分や追跡期間中の曝露差には注意する。

### 重要な制限・今後の拡張
- 生存・繁殖成功度の相関だけでは、毒への耐性が原因であることを断定しない。個体別毒曝露・エネルギー・系譜依存の交絡を追加診断する余地がある。
- 5シードは初期診断。ばらつきが大きい場合はシード数を増やす。絶滅個体群を生存群だけの平均値で代表させない。
- Phase 07.3は色と毒の認識・学習。Phase 07.4は即効毒A、継続毒B、高濃度毒C、遅効毒Dと、それぞれ独立の遺伝性耐性・維持コストを実装する。全耐性最大が常に最適とは限らず、環境によって異なる耐性戦略が共存し得ることを目標とする。
- **各フェーズの着手前に目的、変更範囲、対照条件、評価指標、合格条件をREADMEに固定し、結果に合わせて事後変更しない。**

**Phase 07.2.2 実装上の重要事項:** 従来は耐性維持コストと耐性突然変異が `toxin.enabled` にも依存していたため、毒OFF・コストONの対照実験が成立しなかった。今回 `resistance_enabled` のみを条件にする最小変更を実施。従来の毒OFF・耐性OFF条件の挙動は不変。A/B条件でも遺伝子変異を発生させる。これは観測専用変更ではなく、対照実験成立のための限定的なロジック修正。


## 開発運用ルール（Phase 07.2.3より適用）

- フェーズ開始時に「目的」「変更範囲」「合格条件」を固定し、実装後に検証根拠を記録する。
- 課題状態は「未着手」「対応中」「検証待ち」「解決済み」「先送り」「継続観察」の6種類とする。
- 実装完了と検証完了を区別し、解決済みには検証根拠を記す。先送りには理由・再検討フェーズ・次の対応方針を記す。
- 過去の履歴は保持し、下記の最新課題表を現在の状態とする。自然な絶滅・個体数変動を直ちに不具合とはみなさない。
- ルールが矛盾する場合は、独断で統合・変更せず、矛盾箇所・影響・解決案をユーザーへ相談する。

### 最新課題管理表（Phase 07.2.3開始時）

| ID | 課題 | 状態 | 対応/再検討フェーズ | 根拠・理由 |
|---|---|---|---|---|
| EVO-001 | 耐性の遺伝・突然変異・系譜記録 | 解決済み | 07.2.1.1 | 15実験で親子整合性を確認 |
| EVO-002 | 耐性の適応度差と自然選択 | 対応中 | 07.2.3 | 出生時期・シード差の影響が未分離 |
| ECO-001 | 草食絶滅シードの要因診断 | 継続観察 | 07.2.3 | 絶滅自体はバグと判定しない |
| EVO-003 | 4種類の毒と独立耐性 | 先送り | 07.4 | 単一耐性の進化検証を先に完了する |
| UI-001 | Web上の毒量・耐性・HPの個体別表示 | 先送り | 07.3以降に再検討 | 統計検証を優先、可視化範囲は未確定 |

### Phase 07.2.3：自然選択の統計的検証

**目的**：同じ出生時期・同じシード内で耐性値別の生存・繁殖実績を比較し、毒の有無・強さ・維持コストによる選択方向の違いを評価する。絶滅シードを含め遺伝子分布の推移を確認する。

**変更範囲**：`analyze_phase0723.py` とREADMEのみ。Rust、WASM、Web、毒・耐性・繁殖の計算式は変更しない。Phase 07.2.2の6条件×5シード×50,000 Tickのデータを再利用する。

**合格基準**：①親子継承と突然変異の整合性、②耐性値と生存・繁殖の差が環境条件と整合すること、③複数シードで環境に応じた遺伝子分布の変化が観測されること。高耐性が増えること自体を合格条件にしない。効果が不明確な場合は「検証継続」とする。

**実行**：`python .\analyze_phase0723.py --root .\results\phase0722 --out .\results\phase0723\analysis`。前フェーズの結果ZIPから `phase0722` フォルダを `results` に配置する。

**解析上の制約**：個体を独立な実験反復として扱わず、シード単位の比較を行う。低/高耐性のサンプル不足、親子・環境による交絡、絶滅に伴う打ち切りは明示する。

### Phase 07.2.4：自然選択の検出力と交絡要因の調査

**目的（着手時に固定）**：Phase 07.2.2の30実験を再利用し、耐性値の分散・出生コホートの比較可能性・シード間変動・毒曝露と維持コストの測定可能性を評価する。予想どおりの進化を起こすパラメータ調整は行わない。

**変更範囲**：`analyze_phase0724.py` とREADMEのみ。Rust/WASM/Webの生態ロジック、実験設定、既存合格条件は変更しない。

**完了条件**：①30実験のデータを読み込める、②シード別の耐性分散・コホート数・適応度差のばらつきを記録できる、③既存CSVで調整可能な交絡と測定不能な交絡を明確に区別できる、④追加観測・追加シードの必要性を根拠とともに提示できる。**これらは解析フェーズの完了条件であり、自然選択成立の合格条件ではない。**

**実行**：`python .\analyze_phase0724.py --root .\results\phase0722 --out .\results\phase0724\analysis`

**既存データの制限**：個体ごとの生涯毒曝露、累積耐性維持コスト、空間・餌資源履歴、繁殖年齢到達時点の情報は記録されていない。死亡イベント時点の毒量は生涯曝露を代表しない。したがってこれらの交絡は今回のCSV再解析だけでは除去できない。シード5本で統計的検出力の保証はできない。

### 最新課題管理表（Phase 07.2.4診断後）

| ID | 課題 | 状態 | 再検討フェーズ | 根拠・次の対応 |
|---|---|---|---|---|
| EVO-001 | 耐性の遺伝・突然変異・系譜 | 解決済み | 07.2.1.1 | 既存系譜検証で親子整合性を確認。変更時は回帰テスト |
| EVO-002 | 自然選択の適応度差・世代変化 | 検証待ち | 07.2.5以降 | 30実験で方向が一貫せず、交絡が未調整。選択なしと断定しない |
| OBS-001 | 個体別の生涯曝露・コスト・環境履歴 | 未着手 | 07.2.5（要相談） | 現在のログでは交絡調整不能。観測専用変更を提案 |
| OBS-002 | 追加シードによる検出力改善 | 先送り | OBS-001評価後 | 測定不足のままシードだけ増やしても因果評価は改善しない |
| ECO-001 | 草食の絶滅原因 | 継続観察 | 07.2.5以降 | 絶滅自体はバグとしない。診断可能性を向上する |
| EVO-003 | 4種類の毒と独立耐性 | 先送り | 07.4 | 単一毒・耐性の検証を優先 |
| UI-001 | 毒量・耐性の個体別Web表示 | 先送り | 07.3以降 | 統計・観測基盤を優先 |

**ルール整合性**：既存の「自然選択の合格条件3項目」は変更しない。Phase 07.2.4の完了と自然選択の成立を混同しない。今後ルール矛盾が見つかった場合は、変更前にユーザーに相談する。

## Phase 07.2.5 — 個体別の毒曝露・耐性コスト・繁殖機会（2026-11-03）

**目的:** 自然選択の因果的な説明を妨げている個体別の観測不足を補う。**変更範囲:** 観測用の個体内累積値、CSV出力、実行・解析スクリプトのみ。毒、資源、遺伝、繁殖の意思決定式は変更しない。

**測定:** 累積毒摂取量、毒HP損失、耐性によるHP損失回避、実際に支払った維持エネルギー、繁殖可能年齢到達、成功出生数、死亡/観測打ち切り。`*.individual-life.csv` は死亡時または実験終了時に1個体1行を出力する。生存個体の寿命は右打ち切りとして扱う。繁殖可能年齢の到達は年齢条件のみを意味し、他の繁殖ゲートを満たすことは意味しない。

**検証条件:** 6条件 × 5シード × 50,000 Tick（geneticのみ）。まず1シード×1,000 Tickで動作確認する。毒OFFで累積曝露・毒ダメージがゼロ、コストOFFで維持コストがゼロ、集計が非負、出生数が系譜と整合することを確認する。観測追加による既存診断と生態の一致も比較する。検証前は合格としない。

**課題状態:** 個体別観測不足=対応中（07.2.5）、自然選択の成立判定=検証待ち（07.2.x）、4毒・個別耐性=先送り（07.4）。先送り理由: 単一毒の遺伝・適応度の検証を優先するため。解決済みとするには検証ログが必要。

**課題管理ルール:** 各フェーズの目的・変更範囲・合格条件を開始時に固定する。状態は「未着手／対応中／検証待ち／解決済み／先送り／継続観察」の6種類。解決済みには検証根拠、先送りには理由と再検討フェーズを記す。履歴を保持し、現在の課題一覧を正式状態とする。自然現象とバグを区別する。既存ルールとの矛盾を発見したら独断で統合・削除・変更せず、ユーザーへ相談する。


### Phase 07.2.5.1: 初期草食個体の観測フィールド初期化修正
- 状態: 検証待ち（`cargo test --workspace` 未実行）
- 原因: `Herbivore` に追加した6フィールドを `world/state.rs` の初期個体生成時に設定していなかった。
- 対応: 6フィールドを0またはfalseで初期化。子個体生成は既に対応済み。
- 変更範囲: 初期個体生成のみ。毒・耐性・繁殖の計算式は変更しない。
- 合格条件: `cargo test --workspace` 成功、短時間実験で個体別CSVが生成されること。

### Phase 07.2.5.2: Python集計エラー修正（2026-11-03）
- **状態:** 検証待ち（Windows実環境での再実行待ち）。
- **症状:** 6条件×1シード×1,000 TickのRust実行は完了したが、`analyze_phase0725.py` の `statistics.mean()` が `StatisticsError: mean requires at least one data point` で停止。
- **変更範囲:** Python解析スクリプトのみ。`statistics.mean` を明示的な `sum / n` に変更し、入力CSVの存在・必須列・行値を検証。データがない場合は原因を表示する。Rust・生態・遺伝・繁殖のロジックは変更しない。
- **合格条件:** 既存の短時間実験CSVを解析でき、`results/phase0725/analysis/individual_summary.csv` と `REPORT.md` が生成されること。引き続き `cargo test --workspace` と観測整合性の検証が必要。


## Phase 07.2.6 — 自然選択の再評価（解析のみ）
目的: Phase 07.2.5の個体別終了記録を用い、出生コホートを揃えた固定期間生存差を評価する。
変更範囲: Python解析と資料整理のみ。Rustの生態・進化ロジックは変更しない。
合格基準: (1) 遺伝性の整合性 (2) 毒・コスト条件に整合する適応度差 (3) 複数シードで環境依存的な世代間変化。期待方向への平均値変化のみでは合格にしない。
実行: `python tools/analyze_phase0726.py --root results/phase0725 --out results/phase0726/analysis`。
既存データでは生涯出生数・累積曝露の時系列がなく、固定追跡期間内の繁殖率や曝露調整した因果推定はできない。これを未解決の観測課題として残す。

### 課題管理・変更ルール
各フェーズ開始時に目的・変更範囲・合格条件を固定する。課題の状態は「未着手・対応中・検証待ち・解決済み・先送り・継続観察」。解決済みには検証根拠、先送りには理由と再検討フェーズを記載する。自然な絶滅とソフトウェアの不具合を区別する。履歴を保持し、現行状態を課題一覧で管理する。ルール間の矛盾は独断で変更せず相談する。

| ID | 課題 | 状態 | 根拠・次の対応 |
|---|---|---|---|
| OBS-0725 | 個体別生涯計測 | 解決済み | 30条件実験でCSV生成・毒/コストOFF整合確認 |
| EVO-0726 | 自然選択の統計的判定 | 検証待ち | Phase 07.2.6解析を確認後に判断 |
| OBS-0726 | 個体別の出生イベント時刻・曝露時系列不足 | 先送り | 本フェーズは解析限定。次の観測機能追加フェーズで再検討 |
| EVO-074 | 4種類の毒と独立耐性 | 先送り | 単一毒の自然選択評価を優先しPhase 07.4で再検討 |

### ファイル配置
`README.md`はルート、過去の検証用`.py`/`.ps1`は`tools/archive/`、過去のMarkdownは`docs/history/`に保管。現行解析は`tools/`。
**互換性の注意**: 旧スクリプトにはルート相対パスや`$PSScriptRoot`依存があるため、アーカイブ内の旧ツールは動作保証しない。現行`run_phase0725.ps1`と`analyze_phase0725.py`は既存の実行経路を維持するためルートに暫定残置。移行・回帰テスト後に`tools/`へ移動する。

## Phase 07.2.7 — 個体別イベント時系列の観測（検証待ち）

- 目的: 出生後の同一観測期間における生存・繁殖・毒曝露・毒ダメージ・防御量・耐性維持コストを比較する。
- 変更範囲: `resistance_evolution_0727` の観測用example、`tools/run_phase0727.ps1`、`tools/analyze_phase0727.py` のみ。生態・遺伝・繁殖の判定ロジックは変更しない。
- 記録: `<prefix>.individual-events.csv` に birth / reproduction / toxin_dose / toxin_damage / prevented_damage / resistance_cost / death / censored をTick単位で保存する。既存の個体生涯CSV等も同時出力。
- 成功条件: 30実験の完了、全条件でのイベントCSV生成、毒OFF・コストOFFのゼロ確認、同じ出生コホートでの固定追跡比較、シード別結果の提示。
- 注意: イベントの累積カウンタ差分を毎Tick記録するため、イベントの発生Tickは各step終了後の観測Tick。死亡個体は前回生存時点の最終値を使うため、死亡Tickに生じた曝露・コストを記録できない場合がある。この観測欠落は継続課題であり、精密な死亡直前観測が必要ならコア側のイベントフックを別フェーズで検討する。
- 未解決: 時系列の因果関係・遺伝的浮動の識別、シード間ばらつき、死亡Tick内のイベント欠落。自然選択の成立判定は保留。
- 実行（Windows PowerShell、プロジェクト直下）:
  - `Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force`
  - `./tools/run_phase0727.ps1 -Ticks 1000 -SeedCount 1 -Force`
  - `python ./tools/analyze_phase0727.py --root ./results/phase0727 --out ./results/phase0727/analysis`
  - 標準: `./tools/run_phase0727.ps1 -Ticks 50000 -SeedCount 5 -Force`
- 過去の検証ツールは `tools/archive/`、過去の設計記録は `docs/history/` に保管する。現行の実行入口を優先し、旧ツールの相対パスは完全互換を保証しない。

## Phase 07.2.8 — 選択圧の検出力と交絡要因の検証

- 状態: **解析完了／自然選択の成立は継続検証**。
- 入力: `results/phase0727/analysis/cohort_comparisons.csv`（個体イベント原本の再転送不要）。
- 実行: `python .\tools\analyze_phase0728.py --input .\results\phase0727\analysis\cohort_comparisons.csv --out .\results\phase0728\analysis`
- 出力: `seed_summary.csv`, `condition_uncertainty.csv`, `paired_contrasts.csv`, `confounding_audit.csv`, `quality.json`, `REPORT.md`。
- 30実験・247コホートで解析実行確認、追加品質警告0。5シードの区間は探索的であり、自然選択の因果成立は未確認。
- 詳細・判定・継続課題: `docs/history/phase07/PHASE_07_2_8.md`。
- フォルダ整理: 現行検証ツールは `tools/`、過去の検証ツールは `tools/archive/`、過去のフェーズ資料は `docs/history/` に置く。既存アーカイブの動作保証は別途検証。


## Phase 07.2.9 — 選択圧の検出力改善に向けた実験設計（2026-11-03）

- 状態: **検証待ち**（解析スクリプト実行済み、効果量・主要比較・必要シード数の承認待ち）。
- 目的: Phase 07.2.8の5シード予備統計を用い、事前に固定する実験設計と必要シード数の**概算**を作成。生態・遺伝ロジックは変更しない。
- 入力: `results/phase0728/analysis/seed_summary.csv`。元の10GBイベントCSVは不要。
- 実行: `python .\tools\analyze_phase0729.py --input .\results\phase0728\analysis\seed_summary.csv --out .\results\phase0729\analysis`
- 出力: `results/phase0729/analysis/{planning_scenarios.csv,quality.json,REPORT.md}`。
- 過去ツール: Phase 07.2.8解析を `tools/archive/phase07/` に移動。現行の07.2.7再現ツールは保持。
- 注意: 5シード標準偏差からの正規近似は検出力保証ではない。複数比較・非無作為群・シード不足の課題は未解決。
- 判定基準（未確定）: 主要比較、最小効果量、シード数と途中停止規則を承認してから追加実験を行う。

## Phase 07.2.10 — 追加予備実験と分散再推定（2026-11-03）

- 状態: **検証待ち**（ユーザー承認: 最小検出効果量2パーセントポイント）。
- 目的: 既存5シードに15シードを追加し、F−B条件間差の分散を再推定。F−Bは**確証的主要比較としては未承認**。
- 対象: B毒OFF/コストON、F強毒/コストON。追加シード12350〜12364、各50,000 Tick（30ラン）。
- Rustの生態・進化ロジック変更なし。既存シードは再実行しない。
- 実行: `powershell -ExecutionPolicy Bypass -File .\tools\run_phase07210.ps1 -Ticks 50000 -StartSeed 12350 -SeedCount 15`。
- 解析: `python .\tools\analyze_phase0727.py --root .\results\phase07210\runs --out .\results\phase07210\analysis`、続いて `python .\tools\analyze_phase07210.py --pilot .\results\phase0728\analysis\seed_summary.csv --new .\results\phase07210\analysis\cohort_comparisons.csv --out .\results\phase07210\analysis\design`。
- 共有: `results/phase07210/analysis` のみ。巨大な個体別イベントCSVはローカル保存。
- 設計・品質・判定・未解決課題: `docs/history/phase07/PHASE_07_2_10_DESIGN.md`。

## Phase 07.2.11 — 選択圧のばらつきの原因分析

- 状態: **解析実装・集計検証完了／因果的原因の同定は継続検証**。
- 主要比較: F−B、出生後1,000 Tick生存率差。最小検出対象は2パーセントポイントのまま変更なし。
- `tools/analyze_phase07211.py` は Phase 07.2.10 の軽量CSVのみを入力とし、20ペアシードの1シード除外感度、新規15ペアのコホート構成、毒曝露・維持コストと生存率差の探索的相関を出力する。
- `results/phase07211/analysis/REPORT.md` に観測結果と制約を記録。
- 強い相関は生存期間・摂食機会の交絡や逆因果を排除できない。初期遺伝子分布・捕食圧・個体群規模の因果的寄与は、現在の集計だけでは判定不可。
- 旧5シードのコホート詳細は今回の新規15シードのCSVに含まれないため、シード間の詳細相関は新規15シードのみ。
- 既存の実験を再実行する必要はない。次フェーズで不足するシード別の軽量な観測指標を追加抽出する方針。
- 実行: `python tools/analyze_phase07211.py --paired results/phase07210/analysis/design/paired_seed_summary.csv --cohorts results/phase07210/analysis/cohort_comparisons.csv --out results/phase07211/analysis`


## Phase 07.2.12 — 軽量な初期遺伝子・個体群・捕食圧観測（検証待ち）
- 設計履歴: `docs/history/phase07/PHASE_07_2_12_DESIGN.md`
- 現行ツール: `tools/run_phase07212.ps1`, `tools/analyze_phase07212.py`
- Rust観測専用例: `crates/sim-core/examples/resistance_covariates_07212.rs`
- 追加した出力は `initial-genetics.csv` と `population-pressure.csv` のみ。10GB級イベントCSVは生成しない。
- 初期耐性は現行コード上、設定値で固定される。シード間の初期遺伝子差は存在すると仮定しない。
- 状態: **検証待ち**。既存のF−B結果との整合性、Rustビルド、実機動作は未確認。


## Phase 07.2.12.1 — 観測データの整合性修正（実機検証待ち）

`analysis(2).zip` によると seed=12345 の B/F 観測だけが 1,000 Tick、他の38実験は50,000 Tickでした。旧解析はB/F相互の長さのみを照合し、異なるシード間の実行長の不一致を検出できませんでした。

1. `./tools/repair_phase072121.ps1 -DryRun` で対象を確認。
2. `./tools/repair_phase072121.ps1` で **12345のB/Fだけ** 50,000 Tickに再実行（`-Force` で既存ファイルを置換）。
3. `python ./tools/analyze_phase07212.py --root ./results/phase07212/runs --paired ./results/phase07210/analysis/design/paired_seed_summary.csv --out ./results/phase072121/analysis --expected-ticks 50000` で再解析。
4. `quality.json` の `issues=[]`, `paired_seeds_joined=20`, `condition_runs=40`, `expected_ticks=50000` を確認。

新しい解析は短い実行を明示的にエラーとして除外し、対応するペアが不足すれば非ゼロ終了します。`-Force` を使わない通常実行も短い既存ファイルを黙ってスキップしません。既存のPhase 07.2.10の生存率差は再計算しません。Rustの生態ロジックは変更しません。


## Phase 07.2.13 — 時系列の急減・ゼロ個体群診断（検証待ち）

Phase 07.2.12.1で取得した40ランの `*.population-pressure.csv` を再利用する。**再シミュレーション不要**。生態・遺伝・毒ロジックは変更なし。

```powershell
python .\tools\analyze_phase07213.py --root .\results\phase07212\runs --out .\results\phase07213\analysis --expected-ticks 50000 --window 1000 --drop-fraction 0.30
```

出力: `run_events.csv`、`interval_rates.csv`、`paired_event_timing.csv`、`run_summary.csv`、`quality.json`、`REPORT.md`。品質問題がある場合は終了コード2。

**判定:** 実装済み・40ランのローカル時系列解析は検証待ち。観測窓での急減は診断用の定義であり、因果関係や不可逆な絶滅の証明ではない。詳細は `docs/history/phase07/PHASE_07_2_13_DESIGN.md`。


## Phase 07.2.14 — 急減後の回復・無出生区間の診断（解析実装・共有データ検証済み）

- 目的: Phase 07.2.13で観測された3,000〜6,000 Tick付近の出生ゼロと、その後の個体数回復の関係を記述的に調査。
- 生態・遺伝・毒のRustコードは変更しない。40ランの再実行不要。
- 入力: `results/phase07213/analysis/interval_rates.csv`。共有済み軽量集計CSVを同梱。
- 実行: `python .\tools\analyze_phase07214.py --input .\results\phase07213\analysis --out .\results\phase07214\analysis`。
- 出力: `run_recovery.csv`、`paired_recovery.csv`、`fixed_window_rates.csv`、`condition_summary.csv`、`quality.json`、`REPORT.md`。
- 無出生区間: 3,000〜6,000 Tick内、個体数が存在する100 Tick区間の出生0を連続区間として計測。
- 回復の操作的定義: 6,000→10,000 Tickの個体数増加。絶滅の不可逆性や繁殖機構の停止を意味しない。
- 主要仮説（耐性による自然選択）とは別の探索的診断。原因特定には個体の年齢・繁殖適格性・資源・死因の追加観測が必要。
- 詳細: `docs/history/phase07/PHASE_07_2_14_DESIGN.md`。


## Phase 07.2.15 — 繁殖条件・年齢構成・死亡原因の観測強化（検証待ち）

自然選択検証のため、3,000–6,000 Tickの出生停止の機構を追加観測。**Rustの生態ルールは変更なし**。過去40ランの元CSVは未共有であり、新規診断項目はローカルで40ランを再実行して収集する。

```powershell
cargo test --workspace
.\tools\run_phase07215.ps1 -Ticks 1000 -StartSeed 12345 -SeedCount 1 -DryRun
.\tools\run_phase07215.ps1 -Ticks 50000 -StartSeed 12345 -SeedCount 20 -Interval 100
python .\tools\analyze_phase07215.py --root .\results\phase07215\runs --out .\results\phase07215\analysis --expected-ticks 50000
```

結果の `results/phase07215/analysis` のみZIP化して共有。詳細は `docs/history/phase07/PHASE_07_2_15_DESIGN.md`。

## Phase 07.2.16 — 繁殖停止と資源不足の時系列診断

目的・判定条件: `docs/history/phase07/PHASE_07_2_16_DESIGN.md`。
生態ロジックは変更していません。既存Phase07.2.15の40ランの軽量CSVを再利用します。

```powershell
python .\tools\analyze_phase07216.py --root .\results\phase07215\runs --intervals .\results\phase07213\analysis\interval_rates.csv --out .\results\phase07216\analysis --expected-ticks 50000 --start 2500 --end 6500
```

共有用には `results/phase07216/analysis` だけをZIP化してください。
**重要:** 解析は出生・診断条件の時間的関連を調べるものです。植物・土壌栄養の時系列は現状未取得のため、資源不足の直接原因は未判定です。
課題管理: 実装済み解析=検証待ち（Windows実機未検証）、資源不足の直接原因=継続観察。


## Phase 07.2.17 — 資源量と繁殖条件の同一ラン時系列観測

目的: Phase 07.2.16で不足した植物・土壌資源の観測を、繁殖条件と同一ラン・同一seed・同一Tickに統合する。
変更範囲: 新規example `resource_reproduction_07217.rs`、実行スクリプト `tools/run_phase07217.ps1`、解析スクリプト `tools/analyze_phase07217.py`。生態・毒・遺伝・繁殖ロジックは変更しない。

実行（PowerShell、プロジェクトルート）:

```powershell
./tools/run_phase07217.ps1 -Ticks 50000 -StartSeed 12345 -SeedCount 20 -Interval 100
python tools/analyze_phase07217.py
cargo test --workspace
```

合格条件: B/Fそれぞれ20 seed、合計40ラン、100tick連続記録、同一ラン資源・繁殖指標の欠損なし、`quality.json`のissuesが空、既存テスト合格。分析対象は2500–6500tick。

課題管理: 自然発芽（栄養閾値以上で確率的に植物を発生させ、資源保存則を維持）は **先送り・検証待ち**。資源不足が原因と確認されるまでは実装しない。

## Phase 07.2.18 — 植物・草食個体群の周期と回復遅延（観測専用）

**目的:** 植物減少→草食減少→植物回復の時間差と、回復中の草食最低個体数を同一ランで計測し、自然発芽を導入する必要性を判断する。

**変更範囲:** `tools/run_phase07218.ps1` と `tools/analyze_phase07218.py` の追加のみ。既存の `resource_reproduction_07217` 計測器を再利用。生態・繁殖・遺伝・毒・資源保存則のロジックは変更しない。

**実行:** `cargo test --workspace` の後、`./tools/run_phase07218.ps1 -Ticks 50000 -StartSeed 12345 -SeedCount 20 -Interval 100`。結果は `results/phase07218/analysis`。

**合格条件:** B/F各20seed、全tick範囲の100tick記録、ペアseed一致、`quality.json` の `issues` が空。未回復区間は打ち切りとして集計し、回復完了区間と混同しない。

**課題:** 自然発芽は先送り。土壌栄養増加の死体由来割合は現状の総量スナップショットでは判定不可（継続観察）。植物減少が草食捕食圧由来かどうかも捕食死亡・植物出生・草食個体数の時間順序で仮説を評価するが、因果の証明ではない。必要なら次フェーズで資源移送経路別診断と自然発芽ON/OFFの比較を追加する。

## Phase 07.2.19 — Soil-driven spontaneous germination ON/OFF experiment

**Goal:** Test whether shortening plant recovery prevents severe herbivore bottlenecks while retaining population cycles and evolutionary selection. **Scope:** Opt-in simulation rule and paired diagnostics only. Baseline OFF leaves the prior simulation path unchanged.

- ON: every `CheckInterval` ticks, eligible soil cells (nutrients >= `max(SoilThreshold, initial_body_nutrients)`) are sampled with a deterministic per-cell hash; at most `MaxBirths` plants are created per check. Births debit soil nutrients, and initial energy is captured as sunlight in the energy ledger. No nutrients are created. New plants use the existing default plant genotype/appearance (current plant model has no explicit heritable plant genome).
- OFF: no extra births or extra random draws; compare with original 07.2.18 output for regression.
- Run: `cargo test --workspace` then `./tools/run_phase07219.ps1 -Ticks 50000 -StartSeed 12345 -SeedCount 20 -Interval 100` from PowerShell.
- Output: `results/phase07219/runs` and `results/phase07219/analysis` (80 runs, 40 pairs). Keep the original 07.2.18 results separate.
- Trial defaults: soil threshold 20 nutrients/cell; 2% chance per eligible cell every 20 ticks; max 20 births per check. **These are experimental parameters, not calibrated recommendations.**
- Acceptance criteria: workspace tests pass, all 80 runs complete, 40 pairs match, no conservation violations, OFF produces zero spontaneous births, and paired results are analyzed for extinction, bottlenecks and birth rates. Do not adopt ON as default without examining population cycles and evolutionary selection.
- Issue status: Phase 07.2.19 implementation **verification pending**. Spontaneous germination **experimental**. Gene diversity / trait-frequency analysis **follow-up**.

## Phase 07.2.20 — 自然発芽強度と毒耐性進化の両立（実験）

**最終目的は毒耐性の遺伝的進化の観測。** 草食の絶滅回避・植物の増殖は手段であり、目的指標ではない。

- 目的：自然発芽率 OFF / 0.5% / 1% / 2% における、草食の世代交代・毒耐性平均/分散/出生時の親子形質・毒曝露・絶滅の比較。
- 変更範囲：観測用 example、PowerShell 実行器、Python 解析のみ。自然発芽の既存ルールは変更しない。
- 合格条件：B/F 各20seed × 4強度=160ラン、50,000 tick、100tick間隔、同一seed対応、CSVカバレッジ/出生イベント整合、品質エラー0。**これは計測の合格条件であって進化が証明されたことを意味しない。**
- 実行：`cargo test --workspace` → `./tools/run_phase07220.ps1 -Ticks 50000 -StartSeed 12345 -SeedCount 20 -Interval 100`。
- 出力：`results/phase07220/analysis/REPORT.md`、`run_summary.csv`、`paired_comparison.csv`、`resistance_timeseries.csv`、`quality.json`。
- 注意：毒耐性平均の増加のみでは自然選択を立証できない。毒曝露・繁殖の親子差・遺伝的浮動を考慮し、B/F条件を比較する。世代数を直接測る系譜追跡は今後の課題。
- 課題管理：未着手（長期世代系譜の追跡）／検証待ち（本フェーズの160ラン）／継続観察（遺伝的多様性と絶滅リスク）／先送り（自然発芽強度の標準値確定）。

## Phase 07.2.22.1 — life-event CSV recovery and fitness diagnostics

**Primary objective:** observe inherited toxin-resistance evolution. Survival and reproduction are intermediate measurements, not the end goal.

- Repair legacy unquoted `EntityId { index: N, generation: G }` fields by converting to stable `N:G` strings; validate every CSV column and type.
- New Rust output uses `N:G` directly and writes `censored` snapshots for all surviving animals at the final tick. A disappearance is **not** proof of toxin-caused death.
- Analyze existing Phase 07.2.22 results **without rerunning 80 simulations**:
  `python tools/analyze_phase072221.py --zip "D:\path\to\phase07222.zip" --out results/phase072221/analysis --expected-seeds 20`
- Or analyze extracted files: `python tools/analyze_phase072221.py --root results/phase07222/runs --out results/phase072221/analysis --expected-seeds 20`.
- Output: `individual_fitness.csv`, `run_quality.csv`, `resistance_bin_summary.csv`, `quality.json`, `REPORT.md` (UTF-8).
- Original Phase 07.2.22 CSV has no final survivor snapshots. Individuals without disappearance are **unobserved at end**; do not count them as deaths or infer exact lifetime fitness. Baseline individuals may have reproduced before the observation window.
- A new simulation run with the corrected Rust example can record `censored` survivors. The old simulation results should not be overwritten merely to obtain a first descriptive report.
- **Acceptance:** 80 legacy files parse, no column/ID mismatches, no fabricated death causes, and separate censored/unobserved individuals. Do not claim proof of selection until exposure-adjusted and heritability-aware analysis is available.

## Phase 07.2.23 — 毒耐性と繁殖適応度の定量評価

**目的:** 毒耐性が繁殖成功と関連するかを、出生時期を考慮して80ランの個体履歴から評価する。自然選択による世代間進化を最終目的とし、今回の相関を進化の証明と混同しない。

**変更範囲:** Python解析 `tools/analyze_phase07223.py` を追加。Rust生態モデル・設定・RNGは変更しない。

**合格条件:** 80ランの読込、出生個体の抽出、ラン別回帰、ラン単位の信頼区間、毒なしとの対応seed比較、未観測終端の区別、品質レポート出力。`quality.json` の issues が空であること。

**実行:**

```powershell
python tools/analyze_phase07223.py --input results/phase072221/analysis/individual_fitness.csv --out results/phase07223/analysis
```

**課題管理:**

- 解決済み: 旧形式CSVの個体ID復旧（Phase 07.2.22.1）。
- 検証待ち: Windows上での本フェーズ解析スクリプト実行。
- 継続観察: 毒耐性と出生数の関係、seedごとの符号のばらつき。
- 先送り: 曝露・年齢・資源環境の交絡を制御した生存時間解析、親子系譜による世代間選択の直接評価。

**注意:** 出生時点の耐性を用い、初期個体を除外する。終端未観測を死亡と見なさない。毒の累積曝露量は終端での事後測定なので、曝露補正回帰を因果効果と解釈しない。


## Phase 07.2.24 — 毒耐性の選択圧が弱い原因の診断

目的: 毒耐性が生存と繁殖に結びつかない要因を既存個体履歴から観察的に調べる。生態モデルは変更しない。
変更範囲: `tools/analyze_phase07224.py`, `tools/run_phase07224.ps1` と解析結果。
合格条件: 80ラン入力を検証し、seed×出生5000tickコホートで高低耐性を比較し、打ち切り・死因不明・因果推論の限界を明記する。
課題状態: CSV検証=検証待ち、選択圧の機序=継続観察、自然選択の実証=未着手。
実行: `python tools/analyze_phase07224.py --input <individual_fitness.csv> --out results/phase07224/analysis`

## Phase 07.2.26
毒10倍の耐性上昇と繁殖適応度の関係を既存140ランの個体履歴から解析する。
実行方法・合格条件・課題管理は [PHASE07226.md](PHASE07226.md) を参照。

## Phase 07.2.27

既存phase07225.zipを対象に出生後5,000 Tickの固定期間で繁殖適応度を比較します。`python tools/analyze_phase07227.py --zip phase07225.zip --out results/phase07227/analysis`。

## Phase 07.2.28 — 世代別の毒耐性変化と遺伝的選択

`PHASE07228.md` を参照。既存 `phase07225.zip` の140ランを再利用し、親子系譜・世代深度・出生時期別耐性・固定5,000 Tickの繁殖選択差を検証する。

```powershell
.\tools\run_phase07228.ps1 -InputZip "D:\path\to\phase07225.zip"
```

現時点で毒10倍の耐性上昇と繁殖優位性の観察的証拠はあるが、自然選択の因果的な証明は未達。Phase 07.3〜07.4で共通毒基盤へ段階的に移行し、単一毒の再現性を維持する。


## Phase 07.3.2 — 単一毒の生理計算を共通モジュール化
- **目的**: 草食専用コードから、毒摂取量・排出・HP損傷・耐性軽減・耐性維持費の純粋計算を種非依存モジュールへ抽出する。
- **成果物**: `crates/sim-core/src/toxin.rs`、呼出側変更、単体テスト、07.3.1の比較基準CSV、回帰比較スクリプト、フェーズ設計書。
- **変更範囲**: 単一毒の計算責務のみ。植物・草食・肉食の毒保持形質、複数毒、色学習、擬態は対象外。
- **合格条件**: `cargo test --workspace` 成功、`npm run build` 成功、seed12345・10,000Tickの2種類のCSVが旧版と完全一致、資源保存と既存の毒指標に退行がない。
- **移行判定**: Windowsで上記テストと比較を実行するまで「検証待ち」。不一致時は07.3.3へ進まない。
- **課題**: 07.3.1の静的監査ハッシュは旧版固定であり、新版に対して実行しない。共通化後の新しい監査基準は検証完了時に更新する。
- **次**: Phase 07.3.3 種共通の毒データ表現（毒保有・耐性・毒生成を独立に表す設計）。


### Phase 07.3.2.1 Web preset compatibility fix
- Restore Phase 07.3.1.1 `normalizeConfig` / `mergeMissing` for legacy presets and imported JSON.
- Presets without toxin explicitly default to toxin disabled; standard-toxin remains enabled.
- Rust toxin calculations and baseline comparison files are unchanged.
- Verify on Windows: wasm-pack build, npm run build, browser startup.

## Phase 07.3.2 完了判定（2026-11-03）
- Windowsで10,000Tick・seed12345の資源/繁殖CSVはSHA-256一致、個体履歴2,222イベントは行順を無視すると全レコード一致（ユーザー検証）。
- Phase 07.3.2.1のWeb設定互換性修正後、ユーザーがWeb版動作を確認。
- 判定: **解決済み**。個体履歴の行順だけが変わる問題は、重複件数を含む集合比較で対応。

## Phase 07.3.3 共通毒データ構造（検証待ち）
- **目的**: 単一毒の計算結果・資源収支・メモリレイアウトを維持し、種に依存しない毒データ型を定義する。
- **変更範囲**: `toxin.rs` に `ToxinTypeId` / `ToxinProfile` / `ToxinBurden` / `ToxinResistance` / `ToxinProduction` を追加。既存草食動物にはゼロ割当の参照用アダプタを追加。毒種ID=0のみ有効。毒生産・複数毒・肉食への毒作用は未有効化。
- **互換性**: 既存の `toxin_load:f64` と `genes.toxin_resistance:f32` は引き続き正本。旧JSON/CSVの列・実行順・乱数消費を変更しない。生産形質を実個体へ追加するのはPhase 07.4以降。
- **合格条件**: `cargo test --workspace`、seed12345/10,000Tickの資源CSVハッシュ一致と個体履歴全レコード（順序無視、重複数厳密）の一致、`wasm-pack build`、`npm run build`、Web起動、メモリ増分0（既存個体構造体にフィールド追加なし）。
- **課題**: Rust/Web検証=検証待ち、肉食絶滅=継続観察、複数毒と毒生産=先送り（07.4/07.5/08）。
- **検証**: `powershell.exe -ExecutionPolicy Bypass -File ".\tools\verify_phase0733.ps1"`。既存ベースラインを再利用し追加の長期大量実験は不要。
- **移行判定**: ユーザーのWindows検証結果が揃うまで07.3.4へ進まない。


## Phase 07.3.3 完了判定（2026-11-03）
- Windows Rust 62テスト合格、seed12345・10,000Tickの資源CSV SHA-256完全一致、個体履歴全レコード一致（順序無視）。
- ユーザーがWeb版の動作確認を完了。判定: **解決済み**。

## Phase 07.3.4 — 共通毒システムの回帰・長期安定性検証（検証待ち）
- 目的・範囲・合格条件・限界は `docs/phase07/PHASE0734.md`。
- 変更範囲は検証スクリプトと文書のみ。モデルの動作は変更しない。
- 実行: `powershell.exe -ExecutionPolicy Bypass -File ".\tools\verify_phase0734.ps1"`。
- 既存10,000Tick回帰に加え、seed12345・30,000Tick×2回の同一seed再現性と数値整合性を検証。
- 出力: `results/phase0734/REPORT.json`。結果確認後にPhase 07.4へ進む。


## Phase 07.4.1 — 有毒・無毒植物の遺伝形質（実装・検証待ち）

目的: 植物の有毒・無毒形質を個体単位で導入し、子への遺伝と突然変異、毒曝露・生産コストの差を表現する。

変更範囲: `ToxinSettings` に `plant_variation_enabled`（既定 false）、`initial_toxic_plant_fraction`（既定1.0）、`plant_toxic_mutation_probability`（既定0）を追加。`Plant.produces_toxin` は有毒・無毒の二値形質。創始個体と自然発芽は初期割合、種子繁殖は親の形質を継承し確率的に反転。形質判定は独立ハッシュを使い、生態系の乱数列を変更しない。無毒植物を食べても毒曝露せず、毒合成エネルギーを支払わない。耐性・HP損傷式・資源勘定・色は変更しない。

互換性: 標準設定では従来どおり全植物が有毒であり、旧ベースラインを維持する。`config/phase0741-mixed-plants.json` を読み込んだ場合のみ混合集団（初期有毒率50%、出生時反転率1%）となる。既存のWebプリセットは従来動作。

合格条件: `cargo test --workspace`、`tools/verify_phase0734.ps1` による旧回帰・長期安定性、`cargo test -p sim-core --test plant_toxin0741` の新規テスト、Webビルド・起動。旧回帰は標準設定で比較し、混合設定は別の短期実験として扱う。

課題: 混合形質の長期頻度・適応度の検証は Phase 07.4.2、色認知と擬態は Phase 07.6以降。肉食動物の絶滅は継続観察。


## Phase 07.4.2 — 有毒・無毒植物の選択診断

生態系の挙動を変更せず、有毒・無毒植物別の累積出生・加齢死亡・被食死亡・突然変異方向を計測します。`crates/sim-core/examples/plant_selection_0742.rs` が形質頻度・個体数・毒生産コストのCSVを出力します。`tools/run_phase0742.ps1` でRustテスト、旧版回帰比較、10,000Tickの混合集団観測を実行します。詳細: `docs/phase07/PHASE0742.md`。観測上の増減を直ちに自然選択の因果効果と断定しません。

## Phase 07.4.2 完了判定（2026-11-03）
- Windows `cargo test --workspace` 全件合格。`cargo test -p sim-core --test plant_selection0742` は2/2合格。
- 旧版10,000Tick回帰で資源・繁殖CSVのSHA-256一致、個体履歴CSVの全行一致（行順除外）。30,000Tick×2回の同一seed再現性も合格。
- Web版動作確認済み。混合集団の形質別出生・死亡CSVを確認。判定: **解決済み**。
- コストの選択差への因果的影響は未確定であり、Phase 07.4.3で対照比較する。

## Phase 07.4.3 — 毒生産コストと形質選択の比較検証（検証待ち）
- **目的**: 同一seed・同一初期条件のコスト差だけを比較し、有毒率と形質別出生/死亡率の差を測定する。
- **変更範囲**: 検証スクリプト、分析スクリプト、テスト、文書のみ。Rust/Webのシミュレーション本体は変更しない。
- **実験**: seed12345/12346/12347 × 毒生産コスト0/0.002/0.02 × 10,000Tick（合計9run）。追加大量実験は結果を見て判断。
- **指標**: 有毒率のpaired差、形質別出生/加齢死亡/被食死亡のplant-tick当たり率、累積毒合成コスト。自然発芽を含む出生率は親子遺伝の適応度と同一視しない。
- **合格条件**: Rustテスト、分析テスト、旧版回帰比較、9runの完走、`results/phase0743/REPORT.json` の検査、Web版互換性。
- **実行**: `powershell.exe -ExecutionPolicy Bypass -File ".\tools\run_phase0743.ps1"`。詳細は `docs/phase07/PHASE0743.md`。
- **課題**: コスト効果の一般性=検証待ち、捕食者の色学習/ベイツ型擬態=先送り、肉食絶滅=継続観察。

## Phase 07.4.4 — 毒生産コストと植物の繁殖・生存メカニズム

Phase 07.4.3の9runを再利用して、形質別の出生率・老齢死亡率・被食死亡率と草食個体数の時系列を比較します。シミュレーション計算処理は変更しません。詳細は [`docs/phase07/PHASE0744.md`](docs/phase07/PHASE0744.md) を参照してください。

```powershell
powershell.exe -ExecutionPolicy Bypass -File ".\tools\run_phase0744.ps1"
```

植物エネルギーや親由来の繁殖成功率は既存CSVから直接観測できないため、メカニズムの因果的特定は未確定です。

## Phase 07.4.5 — 植物エネルギーと親の繁殖成功観測
`docs/phase07/PHASE_07_4_5.md` 参照。`tools/run_phase0745.ps1` を実行。旧版互換・資源収支を維持し、親形質別の種子散布成功と植物エネルギーを観測。


## Phase 07.5 — 複数毒・種別耐性（実装開始）
生理計算カーネルと単体テストを追加。生態系への接続は未実装。詳細: `docs/phase07/PHASE075_MULTI_TOXIN.md`。


## Phase 07.5.1 — 植物の毒タイプ遺伝子と突然変異
詳細: `docs/phase07/PHASE0751.md`。毒タイプ遺伝子を有毒・無毒形質から独立して導入。初期値は無効、従来毒タイプ0を維持。検証: `tools/verify_phase0751.ps1`。複数毒の摂食時伝播・耐性遺伝子は未実装。

### Phase 07.5.2 — 草食動物の毒タイプ別負荷と耐性遺伝子（検証待ち）

- 目的・変更範囲・合格条件: [`docs/phase07/PHASE0752.md`](docs/phase07/PHASE0752.md)
- 実行: `powershell.exe -ExecutionPolicy Bypass -File ".\tools\verify_phase0752.ps1"`
- 既定で無効。タイプ0の既存経路は維持し、タイプ1以降のみ独立した負荷・耐性を使用。
- 課題: **検証待ち** Rust全テスト・旧版回帰、**未着手** タイプ別の世代進化観測。

### Phase 07.5.3 — 毒タイプ別耐性の世代進化観測（計測実装・検証待ち）
`docs/phase07/PHASE0753.md` を参照。`tools/verify_phase0753.ps1` でRust全テストと旧版回帰、`tools/run_phase0753.ps1` でタイプ別耐性・出生親子・死亡形質のCSVを生成する。**この段階では進化成立を宣言しない。**

### Phase 07.5.4 — 複数毒の選択圧と絶滅要因の診断

診断目的・比較条件・合格条件は `docs/phase07/PHASE0754_DIAGNOSIS.md` を参照。既存シミュレーションロジックは変更せず、4条件×2seedの対照実験を追加。Windows検証待ち。

## Phase 07.5.5 複数毒の耐性維持コスト検証
追加タイプの維持費のみ倍率0/0.5/1で比較。既定値1で旧版互換。詳細は `docs/phase07/PHASE0755_UPKEEP.md`。検証 `tools/verify_phase0755.ps1`、比較 `tools/run_phase0755.ps1`。状態: 検証待ち。


## Phase 07.5.6 — 公平な維持費と遺伝的選択（検証待ち）
詳細: `docs/phase07/PHASE0756_SELECTION.md`。最終目標は毒タイプ別耐性の遺伝的進化の観測。旧版回帰と比較実験を分離して判定する。


## Phase 07.5.7
毒タイプ別選択圧のペア比較: [docs/phase07/PHASE0757.md](docs/phase07/PHASE0757.md)。`tools/verify_phase0757.ps1` → `tools/run_phase0757.ps1`。最終目的は毒A/B耐性の世代間の適応的進化の実証。


## Phase 07.5.8
See `docs/phase07/PHASE0758.md`. Run `tools/verify_phase0758.ps1`, then `tools/run_phase0758.ps1`.


## Phase 07.5.9 — 耐性防御効果の介入比較
詳細: `docs/phase07/PHASE0759.md`。検証: `tools/verify_phase0759.ps1`、実験: `tools/run_phase0759.ps1`。最終目的は毒タイプ別の遺伝的進化の実証。

### Phase 07.5.9 実施記録（2026-11-06）

12条件×10,000Tickを新規実行。旧版回帰・Rust93テスト・Python39テスト、8,538個体の親子/終端/タイプ別イベント照合が合格。世代深度4～8とタイプ別曝露・繁殖・出生コホートを評価した。技術検証・記述評価は完了したが、耐性差の方向はseed/世代で混在し、**適応的な遺伝的進化の実証は未達**。研究判定は検証待ち/継続観察を維持し、次フェーズへ自動移行しない。詳細: [実施結果](docs/phase07/PHASE0759_RESULTS_20261106.md)。
