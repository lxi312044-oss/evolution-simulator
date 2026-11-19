# 段階4の固定実験設定

ユーザー承認により、**毒・色・攻撃を観察する実験用の固定設定ペア**を採用し、用途を限定した統合設定の調整・固定を完了した。採否・完了の正本は [Issue #38](https://github.com/lxi312044-oss/evolution-simulator/issues/38)、条件別の検証結果は [互換監査・組合せ比較](https://github.com/lxi312044-oss/evolution-simulator/issues/38#issuecomment-6821327069)。

## 固定する設定

| 条件 | 肉食の毒損傷閾値／係数 | 設定 | SHA-256 |
| --- | --- | --- | --- |
| 基準 | 0.50／0.01 | [stage4-inherited-plant-colors.json](../config/stage4-inherited-plant-colors.json) | `08337950cbab5623656f7760b7398675f555c20cabe5da81ee2b0da3bbcf0834` |
| 候補 | 0.05／0.50 | [stage4-inherited-predator-candidate.json](../config/stage4-inherited-predator-candidate.json) | `e2e366b1a7bdcf36826972898592f1e3062e18ac008f143767f0321de808ffcc` |

基準の係数0.01は専用項目を省略して共有既定値を参照する。両側とも紫は有毒120・無毒30、緑は有毒30・無毒120で、有毒株のA/Bは各色で均等。種子は親のRGB・毒性・毒型を継承し、この3形質の変異はゼロ。土壌発芽OFF、草食・肉食の色学習ON、草食の空腹時再評価OFF。既存の草食向け用途限定環境の設定はこの基準と同じ。CLI/Web標準と[旧同色・全有毒の回帰基準](../config/stage4-carnivore-threshold-050.json)は維持する。過去の閾値・係数・配色候補の保留はIssueの履歴に残す。

## 実行版と成果物

- 生態ソース・実行器・設定の固定版は [`f8d23f02a75ff520e128614b9fcce8175cfa22b0`](https://github.com/lxi312044-oss/evolution-simulator/commit/f8d23f02a75ff520e128614b9fcce8175cfa22b0)。Gitツリー `6ea37ca2bb07242ae2b48d937a6cb320063870c9`。Cargo.lock SHA-256 `4faad79321bd6fcfd56749a98626716aa7ca7ca27c77d33dd4f7d7e475b3f0e5`。
- 実測はWindowsネイティブの監査付きRust実行器 [stage4_herbivore_learning](../crates/sim-core/examples/stage4_herbivore_learning.rs)。SHA-256 `62dc5727a7b5d3f21b8e40273a8bed583a1665d257994ae0f48cc4a3b1cabaad`。[固定実行スクリプト](../tools/run_stage4_inherited_predator.py)はseed 12345～12347を各6,000 Tick、計6条件で実行。120秒／条件、永続出力10 MiBの上限に対し実績15.79～20.68秒／条件、785,441 B。
- 成果物はローカルの `results/stage4-inherited-predator/`。6件の圧縮JSONと `manifest.json` を照合済み。manifest SHA-256 `3b7dd7454175d4a4862045843125d83ba74e9572578297ec5d9bc147560db2ff`。各ファイルの設定・時間・SHA-256はmanifestに記録。
- 同じコミットのクリーンなエクスポートから参照用のraw WASM `sim_wasm.wasm`（sim-wasm 0.1.0、rustc 1.99.1、wasm-bindgen 0.2.130）を別途ビルドした。SHA-256 `f84879e94164c695a0fb0fbee6fff162e5f4bf28cedae771860305186885588c`、保存先 `results/stage4-fixed-artifacts/sim_wasm.wasm`。ビルドは `cargo build --target wasm32-unknown-unknown --release --locked --offline -p sim-wasm`。固定6条件の証拠はネイティブ実行に由来し、このWASMでの軌道検証・Webパッケージ生成・既存Webへの配置は実施していない。
- 旧0.05基準の共通状態終端ハッシュは全3seed一致し、追加回復状態は全Tickで既定。今回の比較は各条件内の観測有無で500 Tick間隔の状態・乱数が一致し、毎Tickの容量・N/E保存と毒収支の監査に合格。[互換監査スクリプト](../tools/audit_stage4_carnivore_compatibility.py)

## 観察できた経路と代償

- 草食の初期個体は両色を食べられる場面で、両条件とも実摂食の約91～98%で緑を食べた。両条件の草食学習はON。草食学習ON/OFFの比較結果は [Issue #38](https://github.com/lxi312044-oss/evolution-simulator/issues/38) の先行試験に帰属する。
- 肉食の毒移送は両条件で成立。基準では損傷と記憶更新は0件。候補では3seedすべてで、同一肉食個体の実摂食、非致死の損傷、記憶更新、攻撃可能な再遭遇、判断を確認した。色拒否123件のうち112件は経験した獲物、11件は別の同色個体。予備Eが十分なら許容度を超える危険度で実攻撃した例も123件あった。
- 候補の肉食摂食Nは2seedで減少、1seedで増加し、毒によるHPゼロ除去は0/11、0/11、0/9（全肉食除去が分母）。終端肉食は基準→候補で2→1、2→0、0→3。草食の出生・終端数は3seedとも増加し、終端植物N/Eは減少した。結果は閾値と係数を一緒に変更した総効果。

類似色への実拒否、閾値と係数の個別寄与、肉食学習ON/OFFの因果差、長期存続、進化・擬態は未確認。seed 12345～12347は調整に使った探索標本であり、段階5の独立確認標本には含めない。

## 段階5の最初の問い（提案）

**肉食の色学習は、草食の可視色と系譜の繁殖寄与の関係を変えるか。** 関連実験で未使用と確認したseedを使い、候補設定で肉食の学習ON/OFFを比較する問い。今回は研究計画の確定や試験開始は行わない。
