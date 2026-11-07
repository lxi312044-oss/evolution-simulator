# README課題のIssues移管（2026-11-06）

GitHub運用台帳: https://github.com/lxi312044-oss/evolution-simulator/issues/4

README 591行付近「開発運用ルール（Phase 07.2.3より適用）」と後続の課題管理・完了判定を参照し、新規20課題・既存2課題を登録/更新した。解決済みの2課題は根拠を記録してClosed。先送りと継続観察はOpenのまま。別途、使い方と索引の台帳Issue #4を作成した。

既存READMEの履歴・フェーズ合格条件・正式状態は変更していない。過去にIDのない項目には移管IDを付け、歴史上のIDと区別する。実装・実験・CI成功だけで研究課題を解決済みにしない。

| Issue | ID / 移管ID | 登録時の状態 | 課題 |
|---|---|---|---|
| [#1](https://github.com/lxi312044-oss/evolution-simulator/issues/1) | OBS-0726（残課題） | 未着手 | 死亡Tick内イベント計測 |
| [#2](https://github.com/lxi312044-oss/evolution-simulator/issues/2) | ECO-001 | 継続観察 | 草食絶滅要因 |
| [#5](https://github.com/lxi312044-oss/evolution-simulator/issues/5) | EVO-002 / EVO-0726 | 検証待ち | 耐性の適応度差・世代変化と因果的自然選択を検証する |
| [#6](https://github.com/lxi312044-oss/evolution-simulator/issues/6) | OBS-001（残課題） | 検証待ち | 個体別の曝露・維持費・環境履歴の計測範囲を監査する |
| [#7](https://github.com/lxi312044-oss/evolution-simulator/issues/7) | OBS-002 | 先送り | 追加seed数と自然選択検出力を設計する |
| [#8](https://github.com/lxi312044-oss/evolution-simulator/issues/8) | UI-001 | 先送り | Webで毒量・耐性・HPを個体別に表示する |
| [#9](https://github.com/lxi312044-oss/evolution-simulator/issues/9) | EVO-003 / EVO-074 | 先送り | 4種類の毒作用と独立した耐性を段階的に検証する |
| [#10](https://github.com/lxi312044-oss/evolution-simulator/issues/10) | ECO-PLANT | 継続観察 | 植物絶滅・回復遅延と長期共存の条件を診断する |
| [#11](https://github.com/lxi312044-oss/evolution-simulator/issues/11) | ECO-CARNIVORE | 継続観察 | 肉食動物の絶滅・飢餓・繁殖停止を診断する |
| [#12](https://github.com/lxi312044-oss/evolution-simulator/issues/12) | NUM-0551 | 検証待ち | 旧5seedの50,000Tickエネルギー保存則停止を再確認する |
| [#13](https://github.com/lxi312044-oss/evolution-simulator/issues/13) | OBS-RESOURCE | 継続観察 | 土壌栄養と植物回復の資源移送経路を測定する |
| [#14](https://github.com/lxi312044-oss/evolution-simulator/issues/14) | ECO-GERMINATION | 先送り | 自然発芽強度の標準値と進化への影響を確定する |
| [#15](https://github.com/lxi312044-oss/evolution-simulator/issues/15) | OBS-LINEAGE | 検証待ち | 長期系譜・遺伝的多様性とタイプ別耐性進化を追跡する |
| [#16](https://github.com/lxi312044-oss/evolution-simulator/issues/16) | PLANT-COST-0743 | 検証待ち | 毒生産コストと植物の繁殖・生存差を検証する |
| [#17](https://github.com/lxi312044-oss/evolution-simulator/issues/17) | VERIFY-075 | 検証待ち | Phase 07.5各段階の検証証拠と正式ゲートを整理する |
| [#18](https://github.com/lxi312044-oss/evolution-simulator/issues/18) | VERIFY-HISTORY | 検証待ち | READMEに残る過去フェーズの検証待ちを証拠で棚卸しする |
| [#19](https://github.com/lxi312044-oss/evolution-simulator/issues/19) | LEARN-076 | 未着手 | 色の学習・類似色への一般化と捕食回避を実装・検証する |
| [#20](https://github.com/lxi312044-oss/evolution-simulator/issues/20) | MIMIC-077 | 先送り | 植物のベイツ型擬態が自然発生するか検証する |
| [#21](https://github.com/lxi312044-oss/evolution-simulator/issues/21) | ANIMAL-08 | 先送り | 動物の毒生成・蓄積・捕食伝播と擬態を実装する |
| [#22](https://github.com/lxi312044-oss/evolution-simulator/issues/22) | COEV-09 | 先送り | 毒・色・耐性・識別能力の共進化を検証する |
| [#23](https://github.com/lxi312044-oss/evolution-simulator/issues/23) | EVO-001 | 解決済み | 耐性の遺伝・突然変異・系譜記録の整合性（履歴移管） |
| [#24](https://github.com/lxi312044-oss/evolution-simulator/issues/24) | OBS-0725 | 解決済み | 個体別生涯計測の整合性（履歴移管） |

使い方、状態遷移、検索例、根拠の記録要件はIssue #4を参照。状態の変更時はIssueの履歴に日付・理由を追記し、READMEと矛盾する場合は無断で変更せず相談する。本書は移管時点の記録であり、最新状態は各IssueとREADMEの対応を確認する。

今回はGitHub Issuesへの登録・更新とこのローカル記録のみ。リポジトリへのcommit/push/PR作成やシミュレーション変更は行っていない。
