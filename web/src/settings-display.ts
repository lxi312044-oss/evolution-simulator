// Explicit display metadata. The Rust schema and validator own the stored values.
type Setting = { name: string; description: string; range: string };
const ranges: Record<string, string> = {
  b: "ON／OFF", p: "0〜1（0.05＝5%）", u: "整数 0〜4,294,967,295",
  ui: "整数 1〜4,294,967,295", seed: "整数 0〜9,007,199,254,740,991",
  n: "0以上", pos: "0より大きい数",
};
const definitions = `
seed|乱数シード|同じ設定とシードで初期配置と乱数系列を再現します。|seed
world.width|世界の幅|マップの横方向の広さです。|pos 距離
world.height|世界の高さ|マップの縦方向の広さです。|pos 距離
world.land_columns|土壌の列数|土壌を横方向に分割します。行数との積は32ビット整数の上限内にします。|ui セル
world.land_rows|土壌の行数|土壌を縦方向に分割します。列数との積は32ビット整数の上限内にします。|ui セル
world.max_living_entities|全種共通の個体数上限|登録済みの植物・草食・肉食の合計がこの上限を超える生成を止めます。|u 個体
initial_population.plants|初期植物数|開始時の株数です。継承配色は300株、毒性連動配色は偶数株が必要です。|u 株
initial_population.herbivores|初期草食数|開始時の草食個体数です。全種の初期合計は共有上限以内にします。|u 個体
initial_population.carnivores|初期肉食数|開始時の肉食個体数です。肉食がONなら1体以上必要です。|u 個体
simulation.decision_interval_ticks|判断間隔（互換項目）|互換性のため読み取り専用で表示します。現行の生態更新では参照されません。|ui Tick
simulation.max_steps_per_call|1回の更新で進める上限|Webやバッチからの1回の更新呼び出しのTick数を制限します。|ui Tick
plants.color_inheritance_enabled|植物の色の継承・変異|種子は親のRGBを引き継ぎます。初期株と土壌発芽は緑です。実験配色2種とは併用できません。|b
plants.color_mutation_probability|植物の色が変異する確率|種子の出生時にRGB各成分を独立に判定します。毒性・毒型の判定とは独立です。|p
plants.color_mutation_scale|植物の色変異の最大幅|変異した成分に最大幅の範囲で増減を加え、0〜1に収めて8ビット色に丸めます。|0〜1 正規化RGB
plants.initial_soil_nutrients_per_cell|初期土壌栄養|各土壌セルに配置する栄養です。|n N／セル
plants.initial_detritus_per_cell|初期有機物|各セルの有機物に配置する栄養です。|n N／セル
plants.initial_body_nutrients|初期植物の身体栄養|開始時の植物に配る身体Nです。成体身体N以下にします。|n N
plants.initial_energy|初期植物の予備エネルギー|開始時の植物の予備Eです。植物のE上限以下にします。|n E
plants.photosynthesis_per_tick|光合成量|植物が光から取り込むエネルギーの基準量です。|n E／Tick
plants.basal_energy_per_tick|植物の基礎消費|植物の維持に使うエネルギーの基準量です。|n E／Tick
plants.nutrient_uptake_max|土壌栄養の最大吸収量|土壌から1 Tickに取り込める栄養の上限です。|n N／Tick
plants.nutrient_half_saturation|吸収が半分になる土壌栄養|土壌Nがこの値のとき、吸収要求が最大量の半分になります。|pos N
plants.growth_per_tick|植物の成長速度|1 Tickに進められる成長度の上限を調整します。実際の成長は資源にも依存します。|n 成長度／Tick
plants.growth_energy_per_nutrient|植物の成長費|身体Nを増やすために消費するエネルギーです。|n E／N
plants.adult_body_nutrients|成体植物の身体栄養|成長と種子生成の身体N判定に使います。|pos N
plants.energy_capacity|植物の予備E上限|植物が予備に蓄えられるエネルギーの上限です。|pos E
plants.seed_cooldown_ticks|種子生成の待機時間|種子を作った親が次の生成を試せるまでの間隔です。|u Tick
plants.seed_body_nutrients|種子に渡す身体栄養|親の予備Nから種子の身体へ移します。|pos N
plants.seed_energy|種子に渡すエネルギー|親から種子へ移す予備Eです。植物のE上限以下にします。|n E
plants.seed_work_energy|種子生成の作業費|種子生成時に親が消費するエネルギーです。|n E
plants.seed_dispersal_radius|種子の散布半径|親の位置から種子を散布する最大距離です。|n 距離
plants.seed_probability|種子生成を試す確率|成長・待機条件を満たした株の確率判定に使います。資源や容量の条件もあります。|p
plants.max_age_ticks|植物の寿命|この年齢に達した植物は寿命で除去されます。|ui Tick
plants.mineralization_rate|有機物の無機化率|セルの有機物Nを土壌Nへ戻す割合です。|p ／Tick
plants.soil_diffusion_rate|土壌栄養の拡散率|隣接セル間で土壌Nを移す強さです。|p ／Tick
toxin.enabled|植物毒の作用|植物の毒生成費と摂食による草食の曝露・解毒・損傷を有効にします。配色、動物毒、耐性維持費は個別の設定に従います。|b
toxin.dose_per_nutrient|植物を食べたときの毒量|有毒株から食べたNに比例して草食に加わる毒量です。|n 毒量／N
toxin.clearance_fraction_per_tick|毒の解毒率|被毒負荷から毎Tick取り除く割合です。草食と動物毒ON時の肉食で使います。|p ／Tick
toxin.damage_threshold|毒損傷の共有閾値|解毒後の負荷がこれを超えると損傷を計算します。肉食専用閾値があればそちらを使います。|n 毒量
toxin.hp_damage_per_excess_unit|共有の毒損傷係数|閾値超過分から耐性軽減前のHP損傷を計算します。肉食専用係数で上書きできます。|n HP／毒量／Tick
toxin.plant_energy_cost_per_tick|植物毒の生成費|植物毒ON時に有毒株が支払うエネルギーです。|n E／Tick
toxin.plant_variation_enabled|植物の有毒・無毒形質の継承と変異|有毒・無毒の初期差を有効にし、種子は親の形質を継承して設定確率で反転します。両実験配色でONが必要です。|b
toxin.experimental_plant_toxicity_colors|実験用：毒性連動配色|初期株を有毒・無毒のペアにし、有毒性に応じた色を表示します。偶数株と有毒・無毒形質ONが必要で、継承配色とは併用できません。|b
toxin.experimental_inherited_plant_colors|実験用：植物の初期4区分と色の継承|初期300株を色・毒性の4区分に割り当て、種子は親の色を継承します。土壌発芽は停止します。形質と毒型の継承ON、2毒型、両変異率0が必要です。|b
toxin.initial_toxic_plant_fraction|初期・土壌発芽時の有毒植物の割合|有毒・無毒形質ON時に使います。実験専用の初期割当が優先される場合があります。|p
toxin.plant_toxic_mutation_probability|有毒・無毒が反転する確率|種子の生成時に親の有毒・無毒の性質を反転する確率です。継承配色では0にします。|p
toxin.plant_toxin_type_variation_enabled|植物の毒型の継承と変異|植物に毒型を割り当て、種子に引き継ぎます。OFFでは型A（0）を使います。|b
toxin.plant_toxin_type_count|利用する植物の毒型数|植物に割り当て可能な毒型の数です。継承配色は2型固定です。|整数 1〜16 型
toxin.initial_type0_fraction|初期・土壌発芽時の毒Aの割合|2毒型の初期配置と土壌発芽に使います。継承配色の有毒株は各色でA/Bを均等に割り当てます。|p
toxin.plant_toxin_type_mutation_probability|毒型が変わる確率|種子の毒型を親と異なる型へ変える確率です。有毒・無毒の反転は別項目で指定します。|p
toxin.herbivore_multi_toxin_enabled|草食の毒型別の被毒と耐性|毒型ごとに負荷と耐性を扱います。OFFでは単一の負荷を使います。|b
toxin.resistance_enabled|毒耐性と維持費|耐性による損傷軽減・遺伝・維持費を有効にします。維持費は植物毒OFFでも適用されます。|b
toxin.resistance_effectiveness|最大耐性での損傷軽減率|耐性が1のときに防ぐ毒損傷の割合です。|p
toxin.resistance_energy_cost_per_tick|耐性1の維持費|耐性維持に予備Eから支払う基準費用です。|n E／Tick
toxin.additional_type_resistance_upkeep_multiplier|追加毒型の耐性維持費倍率|複数毒型で、毒A以外の耐性維持費に掛けます。|0〜1 倍
toxin.multi_type_resistance_upkeep_multiplier|複数毒型の総維持費倍率|複数毒型を扱うとき、耐性維持費の合計に掛けます。|0〜1 倍
toxin.initial_resistance|初期の毒耐性|初期個体の耐性です。|p
toxin.resistance_mutation_probability|耐性が変異する確率|出生時に親の耐性から変化する確率です。|p
toxin.resistance_mutation_scale|耐性変異の最大幅|変異時に親の耐性から増減できる幅です。|p
animal_toxin.enabled|動物毒の作用|動物の毒生成・捕食による毒移送・肉食の被毒処理を有効にします。肉食の色学習にはONが必要です。|b
animal_toxin.color_learning_enabled|肉食の色学習|摂食後の被毒経験を獲物の可視色に結びつけ、次の選択や攻撃に使います。動物毒ONが必要です。|b
animal_toxin.carnivore_damage_threshold|肉食専用の毒損傷閾値|肉食の毒損傷判定だけに使います。空欄なら共有閾値を使います。|n 毒量・空欄可
animal_toxin.carnivore_hp_damage_per_excess_unit|肉食専用の毒損傷係数|肉食の超過負荷当たりのHP損傷を指定します。空欄なら共有係数を使います。|n HP／毒量／Tick・空欄可
animal_toxin.herbivore_toxin_decay_per_tick|草食の毒の分解率|草食が体に蓄えた毒のうち、毎Tickこの割合が分解されます。毒を保つには作り続ける必要があります。0なら分解しません。|0以上1未満 ／Tick
animal_toxin.carnivore_learning_gain|肉食の色学習の強さ|被毒の経験を色の記憶へ書き込むときの倍率です。1色に記憶できる危険度の上限も同じ倍率になります。HPへの損傷は変わりません。空欄なら1倍です。|pos 倍（10000以下）・空欄可
animal_toxin.herbivore_toxic_initial_fraction|草食の有毒個体の初期割合|草食に有毒・無毒の性質を持たせます。開始時の草食はこの確率で有毒です。無毒の個体は毒を作らず、生成費も払いません。空欄なら全個体が生成能力どおりに毒を作ります。|p 空欄可
animal_toxin.herbivore_toxic_mutation_probability|草食の有毒・無毒の変異確率|子が親と反対の性質（有毒↔無毒）で生まれる確率です。有毒個体の初期割合を指定したときだけ使えます。|p
animal_toxin.plant_toxin_transfer_threshold|植物由来の毒が肉食へ移る量|かまれた草食の植物由来の毒が、種類ごとにこの量以上あるときだけ肉食へ移ります。少ない毒は草食に残ります。0なら量によらず移ります。|n 毒量
herbivores.color_learning_enabled|草食の色学習|植物の可視色と摂食後の毒損傷を結びつけ、餌の選択に使います。|b
herbivores.hungry_color_recheck_enabled|実験用：低エネルギー時の餌の再評価|予備Eが最大の25%未満で可食候補が色だけで全除外されたとき、餌を再評価します。|b
herbivores.hungry_color_recovery_enabled|実験用：餌の再評価を回復まで継続|草食の色学習と低エネルギー時の再評価が有効なとき、予備Eが30%以上になるまで回復状態を維持します。|b
herbivores.color_memory_safe_meal_relief|実験用：安全な摂食で色の危険記憶を弱める|草食の色学習が有効なとき、毒を摂らずに植物を食べたTickごとに、その色に近い記憶の危険度を色の近さに応じてこの割合だけ下げます。0で無効です。|p ／摂食Tick
herbivores.color_memory_decay_per_tick|実験用：色の危険記憶の時間減衰|草食の色学習が有効なとき、記憶した危険度を毎Tickこの割合だけ下げます。0で無効です。|p ／Tick
herbivores.speed_upkeep_slope|実験用：草食の速さの維持コスト|速い草食ほど基礎代謝を多く払います。基礎代謝×（1＋この値×（速さ÷基準の速さ−1））で、基準は初期の速さの範囲の中央です。0で無効です。|n
herbivores.counterattack_energy|実験用：草食の反撃量|草食が噛まれたとき（踏みとどまっているかに関係なく）、肉食のエネルギーをこの値×攻撃性×（体力÷60）だけ減らします。0で無効です。|n E
herbivores.counterattack_learning_enabled|実験用：反撃による肉食の色の学習|肉食の色の学習が有効なとき、反撃を受けた肉食がその草食の色を危険として覚えます。|b
herbivores.counterattack_strength_threshold|実験用：反撃できる強さのしきい値|攻撃性×（体力÷60）がこの値以上の草食だけが反撃します。0で全員が反撃します。|n
herbivores.counterattack_retreat_ticks|実験用：反撃を受けた肉食が手を引くTick数|反撃を受けた肉食は、このTick数のあいだその草食を狙いません。0で無効です。|n Tick
herbivores.speed_mutation_enabled|実験用：草食の速さの突然変異|無効にすると、子の速さは親と同じになります（ほかの形質は変異します）。|b
herbivores.carnivore_offspring_probability|実験用：草食から肉食が生まれる確率|草食が子を産むとき、この確率で子が肉食として生まれます（体の色は親から受け継ぎます）。初期の肉食を0頭にできます。0で無効です。標準では食性の値で肉食が生まれるため0です（食性の値を効かせるときは0が必要）。|p
diet_gene.effects|食性の値を効かせる|ONにすると、子の食性の値が0.5以上なら肉食、0.5未満なら草食として生まれ（肉食の親からも）、食べ物から得る量が値で変わります。OFFでは値は受け継がれるだけで何にも効きません。ONのときは「草食から肉食が生まれる確率」を0にしてください。|b
diet_gene.initial_herbivore|最初の草食の食性の値|開始時の草食の値の中心です（0に近いほど草食向き）。0.5未満で指定します。|0以上0.5未満
diet_gene.initial_carnivore|最初の肉食の食性の値|開始時に肉食がいる場合の値の中心です（1に近いほど肉食向き）。|0.5以上1以下
diet_gene.initial_spread|最初の食性の値のばらつき|開始時の値は「中心±この幅」で一様に決まります（草食は0.5未満、肉食は0.5以上に収めます）。|0〜0.5
diet_gene.mutation_probability|食性の値の変異確率|子の値が親から変わる確率です。|p
diet_gene.mutation_scale|食性の値の変異の幅|値が変わるときの変化の大きさ（正規分布の標準偏差）です。大きいほど境目をまたぐ子が生まれやすくなります。|0〜1
diet_gene.efficiency_exponent|食性の中間の不利さ（k）|草食は植物から（1−値）^k、肉食はかんで値^k の割合だけを得ます。kが大きいほど境目近くの個体が不利です。0なら不利はありません。|0〜10
herbivore_risk.hunger_weight|草食：空腹の重み|肉食の危険と空腹を比べるときの空腹の重みです。「空腹の重み×空腹×(1−恐れ)」が「危険×恐れ」以上なら、その危険を受け入れて植物へ向かいます。恐れは遺伝する恐れと学習した恐れの和です。0なら空腹でも危険を受け入れません。|0〜100
herbivore_risk.flee_until_factor|草食：逃げ続ける距離|肉食を見た場所から、肉食に気づく距離のこの倍だけ離れるまで、危険として扱い続けます（受け入れなければ逃げ続けます）。|1〜10 倍
herbivore_risk.memory_ticks|草食：危険な場所を覚えるTick数|肉食を見た場所を、見えなくなってからこのTick数だけ覚えています。記憶は学習で、遺伝しません。|整数 0〜100,000 Tick
herbivore_risk.plant_danger_factor|草食：植物を危険とみなす範囲|覚えた場所から、肉食に気づく距離のこの倍以内の植物は、その危険を受け入れる場合だけ目標にします。0なら植物の選び方は変わりません。|0〜10 倍
herbivores.initial_body_nutrients|草食の初期身体栄養|初期草食の身体に配る栄養です。|n N
herbivores.initial_reserve_nutrients|草食の初期予備栄養|初期草食の予備に配る栄養です。|n N
herbivores.initial_energy_fraction|草食の初期エネルギー割合|予備E上限のうち、初期草食に配る割合です。|p
herbivores.basal_energy_per_tick|草食の基礎消費|毎Tickの基礎代謝費を調整します。|n E／Tick
herbivores.movement_energy_per_unit|草食の移動費|移動距離に応じて消費するエネルギーです。|n E／距離
herbivores.feeding_energy_gain_fraction|草食が取り込む餌のE割合|摂食時に利用する植物のエネルギー割合です。|p
herbivores.feeding_nutrient_absorption|草食が取り込む餌のN割合|摂食した栄養のうち草食が受け取る割合です。|p
herbivores.feeding_nutrients_per_tick|草食の最大摂食量|1 Tickの摂食量を制限します。植物の残量や胃の空きも影響します。|n N／Tick
herbivores.feeding_range|草食の摂食距離|植物を食べられる距離の上限です。|n 距離
herbivores.starvation_hp_fraction|草食の飢餓損傷率|飢餓時のHP損傷計算に用いる割合です。|n 割合
herbivores.plant_protection_enabled|植物が少ないときの摂食制限|植物数が保護閾値以下になると草食の植物摂食を制限します。|b
herbivores.plant_protection_threshold|摂食を制限する植物数|植物保護ON時、残っている植物数がこの値以下で摂食を止めます。|u 株
herbivores.predator_evasion_enabled|草食の肉食回避|近くの肉食を検知したとき、回避移動を有効にします。|b
herbivores.predator_detection_radius|草食が肉食を検知する半径|肉食回避ON時の検知距離です。|pos 距離（回避ON時）
herbivores.predator_flee_speed_multiplier|草食の逃走速度倍率|肉食回避ON時の逃走速度に掛ける倍率です。|1〜3 倍（回避ON時）
herbivores.satiety_enabled|草食の胃と満腹制御|胃への取り込み・消化・満腹による摂食の停止と再開を有効にします。|b
herbivores.stomach_capacity_n|草食の胃容量|胃に保持できる栄養の上限です。|pos N
herbivores.feeding_stop_fraction|草食が摂食を止める満腹割合|胃容量に対する割合がこの値以上で摂食を止めます。再開割合より大きくします。|p
herbivores.feeding_resume_fraction|草食が摂食を再開する満腹割合|胃容量に対する割合がここまで下がると再開します。停止割合より小さくします。|p
herbivores.digestion_n_per_tick|草食の消化量|胃から1 Tickに取り出せる栄養量です。|n N／Tick
herbivores.body_n_repair_per_tick|草食の身体栄養の補修量|予備Nから身体Nを補う上限です。|n N／Tick
herbivores.max_age_ticks|草食の寿命設定|草食の寿命を決める基準です。|ui Tick
herbivores.max_energy_per_hp|草食のHP当たり予備E容量|成体HPに掛けて予備E上限を求めます。|pos E／HP
herbivores.vision_min|草食の視野下限|草食の視野形質の下限です。視野上限以下にします。|pos 距離
herbivores.vision_max|草食の視野上限|草食の視野形質の上限です。視野下限以上にします。|pos 距離
herbivores.speed_min|草食の移動速度下限|草食の速度形質の下限です。速度上限以下にします。|pos 距離／Tick
herbivores.speed_max|草食の移動速度上限|草食の速度形質の上限です。速度下限以上にします。|pos 距離／Tick
herbivores.satiety_reproduction_enabled|草食の繁殖に満腹継続を要求|草食の繁殖条件に一定期間の満腹を加えます。胃と満腹制御と併せて使います。|b
herbivores.reproduction_satiety_threshold|草食の繁殖用満腹割合|繁殖に必要な満腹継続を数える基準です。|p
herbivores.reproduction_well_fed_ticks|草食の繁殖用満腹継続時間|満腹条件を何Tick連続で満たす必要があるか指定します。|ui Tick
herbivores.resource_dependent_reproduction|草食の繁殖を周辺の餌に連動|周辺植物数に応じて繁殖の確率を調整します。|b
herbivores.reproduction_food_radius|草食の繁殖用の餌調査半径|資源連動繁殖で周辺植物を数える半径です。|pos 距離
herbivores.reproduction_food_target|草食の繁殖確率が上限に達する植物数|資源連動繁殖で繁殖確率を計算する基準株数です。|ui 株
herbivores.reproduction_enabled|草食の繁殖|草食が条件を満たしたときの出生を有効にします。|b
herbivores.reproduction_cooldown_ticks|草食の繁殖後待機時間|出生後に次の繁殖を試せるまでの間隔です。|ui Tick
herbivores.reproduction_min_age|草食の最低繁殖年齢|繁殖できる年齢の下限です。|u Tick
herbivores.reproduction_min_hp_fraction|草食の最低繁殖HP割合|成体最大HPに対して必要なHP割合です。|p
herbivores.reproduction_min_energy_fraction|草食の最低繁殖E割合|予備E上限に対して必要な予備E割合です。|p
herbivores.offspring_body_nutrients|草食の子に渡す身体栄養|出生時に子の身体へ移す栄養です。|pos N
herbivores.offspring_reserve_nutrients|草食の子に渡す予備栄養|出生時に子の予備へ移す栄養です。|n N
herbivores.offspring_energy|草食の子に渡すエネルギー|出生時に子へ渡す予備Eです。HP当たり予備E容量の200倍以下にします。|n E
herbivores.reproduction_work_energy|草食の繁殖作業費|子への移送に加え、繁殖で消費するエネルギーです。|n E
herbivores.mutation_probability|草食の形質変異確率|出生時の草食形質の変異に使います。毒耐性の変異確率は別項目です。|p
herbivores.mutation_scale|草食の形質変異幅|草食形質が変異するときの変化幅を調整します。|p
carnivores.enabled|肉食動物|肉食の初期配置と更新を有効にします。ON時は初期肉食が1体以上必要です。|b
carnivores.speed|肉食の移動速度|肉食が1 Tickで移動できる距離の基準です。|pos 距離／Tick
carnivores.vision|肉食の視野|獲物を探す半径です。|pos 距離
carnivores.attack_range|肉食の攻撃距離|獲物に攻撃が届く最大距離です。|pos 距離
carnivores.basal_energy_per_tick|肉食の基礎消費|毎Tickの基礎代謝費を調整します。|n E／Tick
carnivores.movement_energy_per_unit|肉食の移動費|移動距離に応じたエネルギー消費です。|n E／距離
carnivores.initial_body_nutrients|肉食の初期身体栄養|初期肉食の身体に配る栄養です。|n N
carnivores.initial_reserve_nutrients|肉食の初期予備栄養|初期肉食の予備に配る栄養です。|n N
carnivores.initial_energy|肉食の初期エネルギー|初期肉食の予備に配るエネルギーです。|n E
carnivores.body_n_repair_per_tick|肉食の身体栄養の補修量|予備Nから身体Nを補う上限です。|n N／Tick
carnivores.bite_nutrients|肉食の一口の栄養量|攻撃時に獲物から取り込める栄養の基準です。|n N／攻撃
carnivores.bite_energy|肉食の一口のエネルギー量|攻撃時に獲物から取り込めるエネルギーの基準です。|n E／攻撃
carnivores.max_age_ticks|肉食の寿命設定|肉食の寿命を決める基準です。|ui Tick
carnivores.attack_cooldown_ticks|肉食の攻撃間隔|攻撃後に次の攻撃を試せるまでの待機時間です。|u Tick
carnivores.prey_refuge_threshold|獲物保護を発動する草食数|獲物保護ON時、草食数がこの値以下なら攻撃を止めます。|u 個体
carnivores.satiety_enabled|肉食の旧方式の満腹待機|消化がOFFのとき、捕殺後の満腹待機を有効にします。|b
carnivores.satiety_ticks_on_kill|捕殺後の満腹時間|捕殺後に設定される満腹待機時間です。|u Tick
carnivores.kill_bonus_energy|捕殺時の追加摂食E|捕殺時に獲物から追加で取り込めるエネルギーです。|n E
carnivores.kill_bonus_nutrients|捕殺時の追加摂食N|捕殺時に獲物から追加で取り込める身体栄養です。|n N
carnivores.digestion_enabled|肉食の胃と消化|胃の消化と満腹割合による摂食制御を有効にします。|b
carnivores.all_bites_to_stomach|肉食の一口をすべて胃へ送る|通常の攻撃で得る栄養とエネルギーを胃に入れます。消化ONが必要です。|b
carnivores.stomach_capacity_energy|肉食の胃のE容量|胃に保持できるエネルギー量です。|pos E（消化ON時）
carnivores.stomach_capacity_nutrients|肉食の胃のN容量|胃に保持できる栄養量です。|pos N（消化ON時）
carnivores.digestion_energy_per_tick|肉食のE消化量|胃から1 Tickに取り出すエネルギーの基準量です。|pos E／Tick（消化ON時）
carnivores.digestion_nutrients_per_tick|肉食のN消化量|胃から1 Tickに取り出す栄養量です。|pos N／Tick（消化ON時）
carnivores.adaptive_digestion_enabled|肉食の緊急消化|予備Eが少ない場合に消化を速めます。消化ONが必要です。|b
carnivores.emergency_reserve_energy|緊急消化を使う予備E|予備Eがこの値未満のとき緊急消化を検討します。|n E（緊急消化ON時）
carnivores.emergency_digestion_multiplier|緊急消化の倍率|緊急時に消化量に掛ける倍率です。|1以上 倍（緊急消化ON時）
carnivores.feeding_stop_fraction|肉食が摂食を止める満腹割合|消化ON時の停止ラインです。再開割合より大きくします。|p
carnivores.feeding_resume_fraction|肉食が摂食を再開する満腹割合|消化ON時の再開ラインです。停止割合より小さくします。|p
carnivores.reproduction_stomach_max_fraction|肉食の繁殖時の胃充足上限|消化ON時、胃の充足割合がこの値を超えると繁殖を止めます。|p
carnivores.hunting_hunger_mode|肉食の空腹時の狩り方|0は例外なし、1は予備E、2は胃E込みの消費予測で狩り再開を判断します。|整数 0〜2
carnivores.hunting_low_reserve_energy|狩りを再開する低予備E|空腹時の狩り方1で使う予備E閾値です。|n E
carnivores.hunting_forecast_ticks|狩り判断の消費予測期間|空腹時の狩り方2で何Tick先までの消費を見込むか指定します。|ui Tick
carnivores.movement_strategy|肉食の探索方式|0は従来、1は待ち伏せ、2は巡回、3は低E時の待ち伏せ、4は形質に応じた待ち伏せです。|整数 0〜4
carnivores.ambush_rest_ticks|待ち伏せの休止時間|待ち伏せ周期の休止側を定めます。方式4では移動時間との合計が形質依存周期になります。|ui Tick
carnivores.ambush_move_ticks|待ち伏せの移動時間|待ち伏せ周期の移動側を定めます。|ui Tick
carnivores.hybrid_low_energy|低E時の待ち伏せ閾値|探索方式3で待ち伏せに切り替える予備Eです。|n E
carnivores.persistent_search_enabled|肉食の継続探索|探索方向や、見失った獲物の方向を保持する探索を有効にします。|b
carnivores.search_heading_ticks|肉食の探索方向の保持時間|探索時に進行方向を保持する時間です。継続探索ONなら正の整数が必要です。|u Tick
carnivores.lost_prey_memory_ticks|見失った獲物の方向の保持時間|見失った獲物の方向を追う時間です。継続探索ONなら正の整数が必要です。|u Tick
carnivores.prey_refuge_enabled|草食が少ないときの攻撃制限|草食数が獲物保護閾値以下なら肉食の攻撃を止めます。|b
carnivores.food_limited_reproduction_enabled|肉食の繁殖を獲物の豊富さに連動|肉食数に対する草食数に基づく確率判定を繁殖に加えます。|b
carnivores.prey_per_predator_target|肉食の繁殖確率を調整する獲物比|餌連動の繁殖確率を計算する尺度です。同じ個体数なら値が大きいほど確率が下がります。|pos 草食／肉食
carnivores.sustainable_reproduction_enabled|肉食の繁殖に最低獲物比を要求|肉食1体当たりの草食数が最低値を下回ると繁殖を止めます。|b
carnivores.minimum_prey_per_predator|繁殖に必要な最低獲物比|最低獲物比を使う繁殖制御の閾値です。|pos 草食／肉食（制御ON時）
carnivores.prey_trend_reproduction_enabled|草食の減少を肉食の繁殖へ反映|草食の減少傾向に応じて肉食の繁殖確率を下げます。|b
carnivores.prey_trend_window_ticks|草食の減少傾向を測る間隔|草食数の変化を判定する期間です。|正の整数 Tick
carnivores.prey_decline_penalty|草食減少による繁殖抑制の強さ|草食が減少したときの繁殖確率の低下を調整します。|p
carnivores.reproduction_enabled|肉食の繁殖|肉食が条件を満たしたときの出生を有効にします。|b
carnivores.reproduction_cooldown_ticks|肉食の繁殖待機時間|出生後の待機時間と最低繁殖年齢の判定に使います。|ui Tick
carnivores.offspring_body_nutrients|肉食の子に渡す身体栄養|出生時に子の身体に移す栄養です。|n N
carnivores.offspring_reserve_nutrients|肉食の子に渡す予備栄養|出生時に子の予備に移す栄養です。|n N
carnivores.offspring_energy|肉食の子に渡すエネルギー|出生時に子へ渡す予備Eです。|n E
carnivores.reproduction_parent_reserve_energy|肉食の繁殖後に親が残す予備E|出生費を払った直後に親が保持する予備Eの下限です。胃Eは含みません。|n E
carnivores.reproduction_work_energy|肉食の繁殖作業費|子への移送に加えて繁殖時に消費するエネルギーです。|n E
carnivores.mutation_probability|肉食の形質変異確率|体色と、探索方式4での待ち伏せ形質の変異判定に使います。|p
carnivores.mutation_scale|肉食の形質変異幅|体色や待ち伏せ形質が変異するときの変化幅を調整します。|p
carnivores.speed_evolution_enabled|実験用：肉食の速さの遺伝|有効にすると、肉食の速さが個体ごとに異なり、子へ受け継がれます。無効では全個体が「肉食の移動速度」です。|b
carnivores.initial_speed_spread|実験用：肉食の初期の速さのばらつき|速さの遺伝が有効なとき、初期個体の速さを「肉食の移動速度」の±この割合で一様に決めます。0でばらつきなしです。|p
carnivores.speed_mutation_probability|実験用：肉食の速さの変異確率|速さの遺伝が有効なとき、子の速さが変異する確率です。0で変異なしです。|p
carnivores.speed_mutation_scale|実験用：肉食の速さの変異幅|速さが変異するとき、親の速さの±この割合で一様に変えます。範囲は0.5〜8です。|p
carnivores.speed_upkeep_slope|実験用：肉食の速さの維持コスト|速い肉食ほど基礎代謝を多く払います。基礎代謝×（1＋この値×（速さ÷肉食の移動速度−1））です。0で無効です。|n
carnivores.color_memory_decay_per_tick|実験用：肉食の色の危険記憶の時間減衰|肉食が覚えた色の危険度（毒・反撃）を毎Tickこの割合だけ下げます。0で無効です。|p ／Tick
carnivores.hunt_bout_ticks|実験用：肉食の狩りの区切り|狩りをしたTickがこの数に達するたびに、噛みつきで得たエネルギーと使ったエネルギー（基礎代謝＋移動）を比べ、損をしていたら同じTick数だけ休みます。0で無効です。|n Tick
observation.interval|観測ログの記録間隔|指定したTick間隔でCSV用の統計を記録します。|1以上の整数 Tick
observation.stopAt|観測の終了Tick|指定したTickで進行を止めます。0は無制限です。系譜記録を有効にする場合は正の値が必要です。|0以上の整数 Tick
observation.lineage|草食動物の系譜記録|新規開始から草食動物の系譜イベントを記録します。終了Tickの指定が必要です。|b
ui.field_size|フィールドの広さ|セルの大きさを保って縦横のセル数を変更します。独自サイズは維持を選べます。|20・30・40セルまたは現在値
ui.field_density|広さ変更時の初期個体数|面積に合わせて初期個体数を増減するか、元の個体数を維持します。|密度維持または個体数維持
ui.tuning_preset|簡易調整のプリセット|簡易調整の入力欄に試験用の値を読み込みます。適用するまで実行中の環境は変わりません。|旧設定・回復重視・循環改善・カスタム
`;
export const settingDisplay: Record<string, Setting> = Object.fromEntries(
  definitions.trim().split("\n").map(line => {
    const [key, name, description, range] = line.split("|");
    const [code, ...unit] = range.split(" ");
    return [key, { name, description, range: [ranges[code] ?? code, ...unit].join(" ") }];
  }),
);

// The simple editor and observation controls reuse the same field descriptions.
export const settingControlKeys: Record<string, string> = {
  "field-size": "ui.field_size",
  "field-density": "ui.field_density",
  preset: "ui.tuning_preset",
  growth: "plants.growth_per_tick",
  uptake: "plants.nutrient_uptake_max",
  seed: "plants.seed_probability",
  mineralization: "plants.mineralization_rate",
  diffusion: "plants.soil_diffusion_rate",
  radius: "plants.seed_dispersal_radius",
  cooldown: "plants.seed_cooldown_ticks",
  feeding: "herbivores.feeding_nutrients_per_tick",
  "resource-reproduction": "herbivores.resource_dependent_reproduction",
  "food-radius": "herbivores.reproduction_food_radius",
  "food-target": "herbivores.reproduction_food_target",
  "plant-protection": "herbivores.plant_protection_enabled",
  "plant-protection-threshold": "herbivores.plant_protection_threshold",
  "satiety-enabled": "herbivores.satiety_enabled",
  "satiety-reproduction": "herbivores.satiety_reproduction_enabled",
  "repro-satiety-threshold": "herbivores.reproduction_satiety_threshold",
  "repro-well-fed-ticks": "herbivores.reproduction_well_fed_ticks",
  "stomach-capacity": "herbivores.stomach_capacity_n",
  "satiety-stop": "herbivores.feeding_stop_fraction",
  "satiety-resume": "herbivores.feeding_resume_fraction",
  digestion: "herbivores.digestion_n_per_tick",
  herbivores: "initial_population.herbivores",
  "log-interval": "observation.interval",
  "stop-tick": "observation.stopAt",
  "lineage-enabled": "observation.lineage",
};

const dependencyErrors: Record<string, [string[], string]> = {
  "invalid initial_type0_fraction": [
    ["toxin.initial_type0_fraction"], "0〜1の数値を指定してください。"],
  "invalid plant toxin variation settings": [
    ["toxin.initial_toxic_plant_fraction", "toxin.plant_toxic_mutation_probability"],
    "両方とも0〜1の数値を指定してください。"],
  "invalid plant toxin type settings": [
    ["toxin.plant_toxin_type_count", "toxin.plant_toxin_type_mutation_probability"],
    "毒型数は1〜16の整数、変異確率は0〜1の数値を指定してください。"],
  "invalid satiety reproduction settings": [
    ["herbivores.reproduction_satiety_threshold", "herbivores.reproduction_well_fed_ticks"],
    "満腹度は0〜1、継続時間は1以上の整数を指定してください。"],
  "系譜記録には正の終了Tickが必要です": [
    ["observation.lineage", "observation.stopAt"],
    "終了Tickを1以上にするか、系譜記録をOFFにしてください。"],
  "記録間隔・終了Tickを整数で指定してください": [
    ["observation.interval", "observation.stopAt"],
    "記録間隔は1以上、終了Tickは0以上の整数を指定してください。"],
  "捕食再開ラインは停止ライン未満にしてください": [
    ["herbivores.feeding_resume_fraction", "herbivores.feeding_stop_fraction"],
    "再開割合を停止割合より小さくしてください。"],
  "満腹連動繁殖をONにする場合は満腹度システムもONにしてください": [
    ["herbivores.satiety_reproduction_enabled", "herbivores.satiety_enabled"],
    "満腹度システムをONにするか、満腹連動繁殖をOFFにしてください。"],
  "carnivore color learning requires animal toxin": [
    ["animal_toxin.color_learning_enabled", "animal_toxin.enabled"],
    "動物毒をONにするか、肉食の色学習をOFFにしてください。"],
  "all_bites_to_stomach requires digestion_enabled": [
    ["carnivores.all_bites_to_stomach", "carnivores.digestion_enabled"],
    "肉食の消化をONにするか、一口をすべて胃へ送る設定をOFFにしてください。"],
  "enabled carnivores require nonzero initial population": [
    ["carnivores.enabled", "initial_population.carnivores"],
    "初期肉食数を1以上にするか、肉食動物をOFFにしてください。"],
  "initial population exceeds maximum": [
    ["world.max_living_entities", "initial_population.plants", "initial_population.herbivores", "initial_population.carnivores"],
    "初期個体数の合計を上限以下にしてください。"],
  "invalid satiety settings": [
    ["herbivores.stomach_capacity_n", "herbivores.digestion_n_per_tick", "herbivores.feeding_stop_fraction", "herbivores.feeding_resume_fraction"],
    "胃容量を正、消化量を0以上、再開割合を停止割合より小さくしてください。"],
  "invalid carnivore digestion settings": [
    ["carnivores.stomach_capacity_energy", "carnivores.stomach_capacity_nutrients", "carnivores.digestion_energy_per_tick",
      "carnivores.digestion_nutrients_per_tick", "carnivores.feeding_stop_fraction", "carnivores.feeding_resume_fraction", "carnivores.reproduction_stomach_max_fraction"],
    "消化ON時は胃容量と消化量を正にし、満腹割合を0〜1、再開割合を停止割合未満にしてください。"],
  "invalid adaptive carnivore digestion settings": [
    ["carnivores.digestion_enabled", "carnivores.emergency_reserve_energy", "carnivores.emergency_digestion_multiplier"],
    "緊急消化には消化ON、予備E閾値0以上、消化倍率1以上が必要です。"],
  "invalid carnivore toxin damage threshold": [["animal_toxin.carnivore_damage_threshold"], "0以上の数値にするか空欄にしてください。"],
  "invalid carnivore toxin damage coefficient": [["animal_toxin.carnivore_hp_damage_per_excess_unit"], "0以上の数値にするか空欄にしてください。"],
};

export function formatSettingError(error: unknown): string {
  const original = error instanceof Error ? error.message : String(error);
  const label = (key: string) => settingDisplay[key].name + "（" + key + "）";
  const dependency = dependencyErrors[original];
  if (dependency) return dependency[0].map(label).join("、") + "。 " + dependency[1] + "\n原文 " + original;
  const simpleControl = settingControlKeys[original.split(" は ", 1)[0]];
  if (simpleControl && original.includes(" は ")) return label(simpleControl) + original.slice(original.indexOf(" は "));
  const expanded = original.replace(/\b[a-z][a-z_0-9]*\.[a-z][a-z_0-9]*\b/g,
    key => settingDisplay[key] && !original.includes("（" + key + "）") ? label(key) : key);
  if (expanded !== original) return expanded;
  return /[ぁ-んァ-ン一-龯]/.test(original) ? original
    : "入力値・範囲・依存条件を確認してください。\n原文 " + original;
}
