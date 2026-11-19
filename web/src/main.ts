import { defaultWebPreset, greenFounderPreset, startupWebPreset } from "./initial-preset";
import { formatSettingError } from "./settings-display";
import init, { Simulation } from "./pkg/sim_wasm";
import config from "../../config/standard-toxin.json";
import fixedBaseline from "../../config/stage4-inherited-plant-colors.json";
import fixedCandidate from "../../config/stage4-inherited-predator-candidate.json";
import stage4Baseline from "../../config/stage4-carnivore-threshold-050.json";
import stage4Standard from "../../config/stage4-standard.json";
import control from "../../config/phase06219-control.json";
import plant120 from "../../config/phase06219-plant-120.json";
import plant150 from "../../config/phase06219-plant-150.json";
import herbivore120 from "../../config/phase06219-herbivore-120.json";
import combined from "../../config/phase06219-combined.json";
import phase29control from "../../config/phase06229-control.json";
import phase29hunting from "../../config/phase06229-hunting-only.json";
import phase29fixed from "../../config/phase06229-repair-fixed.json";
import phase29combined from "../../config/phase06229-repair-fixed-hunting.json";
import phase30huntingonly from "../../config/phase06230-hunting-only.json";
import phase30stomachrelaxed from "../../config/phase06230-stomach-relaxed.json";
import phase30digestionadjusted from "../../config/phase06230-digestion-adjusted.json";
import phase30combined from "../../config/phase06230-combined.json";
import phase31control from "../../config/phase06231-control.json";
import phase31basal80 from "../../config/phase06231-basal-80.json";
import phase31movement80 from "../../config/phase06231-movement-80.json";
import phase31combined80 from "../../config/phase06231-combined-80.json";
import p32_0 from "../../config/phase06232-control.json";
import p32_1 from "../../config/phase06232-ambush.json";
import p32_2 from "../../config/phase06232-roaming.json";
import p32_3 from "../../config/phase06232-hybrid.json";
import p32_4 from "../../config/phase06232-hybrid-evasion.json";
import p32_5 from "../../config/phase06232-combined80.json";
import p32_6 from "../../config/phase06232-combined80-evasion.json";
import p35fixed from "../../config/phase06235-fixed.json";
import p35ambush from "../../config/phase06235-ambush.json";
import p35genetic from "../../config/phase06235-genetic.json";
import "./style.css";
import { resizeField } from "./field";
import { creatureShape, drawCreature } from "./sprites";
import { EcosystemMap } from "./map";
import { EcologyPanel, drawMimicry } from "./ecology-panel";
import { describeExistingControls, editor } from "./settings";
const el=(id:string)=>document.getElementById(id)!;
const status=el("status");
const canvas=el("world") as HTMLCanvasElement;
const dev = /^\/dev\/?$/.test(location.pathname);
document.body.dataset.mode = dev ? "dev" : "play";
el("mode-description").textContent = dev ? "開発版 · 設定と観測" : "プレイ版 · 生態系を観察";
const map = new EcosystemMap(canvas, el("hit-list") as HTMLSelectElement, el("individual"), render);
const ecology = new EcologyPanel(el("species-ranking"), el("species-tabs"), el("species-totals"), el("event-log"),
  id => map.focus(id, (index, generation) => simulation.inspect_entity_json(index, generation)),
  (species, n) => JSON.parse(simulation.species_member_json(species, n)));
const populationTrend: number[][] = [];
let priorTick = -1;
let mimicryBucket = -1;
let frameStart = 0;
let frameCount = 0;
let renderMs = 0;
let stepMs = 0;
let observedPresent = [false, false, false];
let lineage: unknown[] | null = null;
let lineageFinished = false;
let latestLineageSample: unknown = null;
let stopAt = 0;
let interval = 100;
let reference: {config: Config; condition: string} | undefined;
const playNames: Record<string, string> = {
  "stage4-standard": "肉食は草食から生まれる（標準）",
  [defaultWebPreset]: "無毒の緑から始まる世界",
  "standard-toxin": "通常毒",
  "stage4-fixed-candidate": "色を覚える捕食者",
  "stage4-fixed-baseline": "色のある草原",
};
const playNotes: Record<string, string> = {
  [defaultWebPreset]: "最初の植物はすべて無毒の緑色です。種子から生まれる子では、毒性と色が独立に変異します。草食は食べた後の被毒経験から色を学習します。色だけで有毒株を正確に見分けられるとは限りません。",
  "standard-toxin": "植物・草食・肉食と資源の循環を観察します。",
  "stage4-fixed-candidate": "色を手掛かりに餌を選ぶ草食と、被毒経験を覚える肉食を観察します。",
  "stage4-fixed-baseline": "色と毒性が部分的に対応する草原で、草食の餌選びを観察します。",
  "stage4-standard": "肉食は最初はいません。草食の子がまれに肉食として生まれ、草食の増減に合わせて肉食も増減します。植物の色は子に受け継がれながら少しずつ変わり、ごくまれに毒の有無も変わります（40×40、Seed 90006）。",
};
const settings = editor(el("settings-groups"), () => el("pending").textContent = "未適用の変更があります");
el("zoom-in").onclick = () => map.scale(1.25);
el("zoom-out").onclick = () => map.scale(0.8);
el("center-map").onclick = () => map.center();
const buttons=[el("step"),el("step100"),el("step500"),el("step1000"),el("play"),el("reset"),el("apply"),el("compare"),el("export-log"),el("export-config"),el("import-config")] as HTMLButtonElement[];
type Config = Omit<typeof config, "herbivores"> & {
  herbivores: typeof config.herbivores & { hungry_color_recheck_enabled?: boolean };
};
// Historical presets predate both the toxin field and newer simulation settings.
// Fill missing schema fields from today's config while retaining all old values.
// Missing toxin is explicitly OFF so old toxin-free experiments stay toxin-free.
function mergeMissing<T>(defaults: T, source: unknown): T {
  if (source === null || typeof source !== "object" || Array.isArray(source) ||
      defaults === null || typeof defaults !== "object" || Array.isArray(defaults)) {
    return (source === undefined ? defaults : source) as T;
  }
  const result: Record<string, unknown> = { ...(defaults as Record<string, unknown>), ...(source as Record<string, unknown>) };
  const input = source as Record<string, unknown>;
  for (const [key, fallback] of Object.entries(defaults)) {
    result[key] = mergeMissing(fallback, input[key]);
  }
  return result as T;
}
function normalizeConfig(source: unknown): Config {
  if (source === null || typeof source !== "object" || Array.isArray(source)) {
    throw new Error("設定JSONはオブジェクトである必要があります");
  }
  const input = source as Record<string, unknown>;
  const merged = mergeMissing(config, source);
  if (input.toxin === undefined) merged.toxin = { ...config.toxin, enabled: false };
  return merged;
}
const phase19Configs:Record<string,Config>=Object.fromEntries(Object.entries({"standard-toxin":config,"control":control,"plant-120":plant120,"plant-150":plant150,"herbivore-120":herbivore120,"combined":combined,"p29-control":phase29control,"p29-hunting":phase29hunting,"p29-fixed":phase29fixed,"p29-combined":phase29combined,"p30-hunting-only":phase30huntingonly,"p30-stomach-relaxed":phase30stomachrelaxed,"p30-digestion-adjusted":phase30digestionadjusted,"p30-combined":phase30combined,"p31-control":phase31control,"p31-basal-80":phase31basal80,"p31-movement-80":phase31movement80,"p31-combined-80":phase31combined80,"p32-control":p32_0,"p32-ambush":p32_1,"p32-roaming":p32_2,"p32-hybrid":p32_3,"p32-hybrid-evasion":p32_4,"p32-combined80":p32_5,"p32-combined80-evasion":p32_6,"p35-fixed":p35fixed,"p35-ambush":p35ambush,"p35-genetic":p35genetic}).map(([name, preset]) => [name, normalizeConfig(preset)]));
phase19Configs[defaultWebPreset] = greenFounderPreset(config);
phase19Configs["stage4-fixed-baseline"] = normalizeConfig(fixedBaseline);
phase19Configs["stage4-fixed-candidate"] = normalizeConfig(fixedCandidate);
phase19Configs["stage4-baseline"] = normalizeConfig(stage4Baseline);
phase19Configs["stage4-standard"] = normalizeConfig(stage4Standard);
phase19Configs["stage4-hungry-recheck"] = normalizeConfig({
  ...stage4Baseline,
  herbivores: { ...stage4Baseline.herbivores, hungry_color_recheck_enabled: true },
});
let activeCondition = startupWebPreset;
let baseline:Config = structuredClone(config);
const presets={
 baseline:{growth:0.003,uptake:0.1,seed:0.35,feeding:1.5,herbivores:80,mineralization:0.002,diffusion:0,radius:70,cooldown:100},
 balanced:{growth:0.009,uptake:0.3,seed:0.65,feeding:0.35,herbivores:80,mineralization:0.002,diffusion:0,radius:70,cooldown:100},
 circulation:{growth:0.009,uptake:0.3,seed:0.65,feeding:0.35,herbivores:80,mineralization:0.012,diffusion:0.08,radius:110,cooldown:100}
};
let activeConfig:Config = structuredClone(baseline);
let simulation:Simulation;
function numberValue(id:string,min:number,max:number):number{
 const input=el(id) as HTMLInputElement;
 const raw=input.value.trim();
 // Firefoxのフォーム復元等で空欄になった場合は、適用中JSONの値へ復旧する。
 const fallback = initialInputValue(id);
 const normalized = raw === "" && fallback !== undefined ? String(fallback) : raw;
 if(raw === "" && fallback !== undefined)input.value=normalized;
 const value=normalized === "" ? NaN : Number(normalized);
 if(!Number.isFinite(value)||value<min||value>max)throw new Error(`${id} は ${min}〜${max} の数値を指定してください（現在値: ${JSON.stringify(raw)}）`);
 return value;
}
function initialInputValue(id:string):number|undefined{
 const c=baseline;
 const values:Record<string,number>={growth:c.plants.growth_per_tick,uptake:c.plants.nutrient_uptake_max,seed:c.plants.seed_probability,mineralization:c.plants.mineralization_rate,diffusion:c.plants.soil_diffusion_rate,radius:c.plants.seed_dispersal_radius,cooldown:c.plants.seed_cooldown_ticks,feeding:c.herbivores.feeding_nutrients_per_tick,herbivores:c.initial_population.herbivores,"food-radius":c.herbivores.reproduction_food_radius,"food-target":c.herbivores.reproduction_food_target,"plant-protection-threshold":c.herbivores.plant_protection_threshold,"repro-satiety-threshold":c.herbivores.reproduction_satiety_threshold,"repro-well-fed-ticks":c.herbivores.reproduction_well_fed_ticks,"stomach-capacity":c.herbivores.stomach_capacity_n,"satiety-stop":c.herbivores.feeding_stop_fraction,"satiety-resume":c.herbivores.feeding_resume_fraction,digestion:c.herbivores.digestion_n_per_tick};
 return values[id];
}
function setPreset(which:keyof typeof presets){
 const p=presets[which];
 for(const [id,v] of Object.entries(p)) (el(id) as HTMLInputElement).value=String(v);
 (el("preset") as HTMLSelectElement).value=which;
}
// Firefox等のフォーム値復元で別項目の値が入る場合があるため、
// WASM初期化前に設定ファイルの初期値を明示的に反映する。
function restoreInitialControls(){
 const h=baseline.herbivores;
 const values:Record<string,number>={
  "food-radius":h.reproduction_food_radius,
  "food-target":h.reproduction_food_target,
  "plant-protection-threshold":h.plant_protection_threshold,
  "repro-satiety-threshold":h.reproduction_satiety_threshold,
  "repro-well-fed-ticks":h.reproduction_well_fed_ticks,
  "stomach-capacity":h.stomach_capacity_n,
  "satiety-stop":h.feeding_stop_fraction,
  "satiety-resume":h.feeding_resume_fraction,
  "digestion":h.digestion_n_per_tick
 };
 for(const [id,value] of Object.entries(values)){
  const input=el(id) as HTMLInputElement;
  input.value=String(value);
 }
 (el("resource-reproduction") as HTMLSelectElement).value=h.resource_dependent_reproduction?"on":"off";
 (el("plant-protection") as HTMLSelectElement).value=h.plant_protection_enabled?"on":"off";
 (el("satiety-enabled") as HTMLSelectElement).value=h.satiety_enabled?"on":"off";
 (el("satiety-reproduction") as HTMLSelectElement).value=h.satiety_reproduction_enabled?"on":"off";
}
function readConfig():Config{
 const c:Config=structuredClone(baseline);
 c.plants.growth_per_tick=numberValue("growth",0,0.1);
 c.plants.nutrient_uptake_max=numberValue("uptake",0,10);
 c.plants.seed_probability=numberValue("seed",0,1);
 c.plants.mineralization_rate=numberValue("mineralization",0,1);
 c.plants.soil_diffusion_rate=numberValue("diffusion",0,1);
 c.plants.seed_dispersal_radius=numberValue("radius",0,500);
 c.plants.seed_cooldown_ticks=numberValue("cooldown",0,1000);
 if(!Number.isInteger(c.plants.seed_cooldown_ticks))throw new Error("種子クールダウンは整数で指定してください");
 c.herbivores.plant_protection_enabled=(el("plant-protection") as HTMLSelectElement).value==="on";
 c.herbivores.plant_protection_threshold=numberValue("plant-protection-threshold",0,5000);
 if(!Number.isInteger(c.herbivores.plant_protection_threshold))throw new Error("植物保護閾値は整数で指定してください");
 c.herbivores.satiety_enabled=(el("satiety-enabled") as HTMLSelectElement).value==="on";
 c.herbivores.stomach_capacity_n=numberValue("stomach-capacity",0.01,1000);
 c.herbivores.feeding_stop_fraction=numberValue("satiety-stop",0.01,1);
 c.herbivores.feeding_resume_fraction=numberValue("satiety-resume",0,0.99);
 c.herbivores.digestion_n_per_tick=numberValue("digestion",0,100);
 if(c.herbivores.feeding_resume_fraction>=c.herbivores.feeding_stop_fraction)throw new Error("捕食再開ラインは停止ライン未満にしてください");
 c.herbivores.satiety_reproduction_enabled=(el("satiety-reproduction") as HTMLSelectElement).value==="on";
 c.herbivores.reproduction_satiety_threshold=numberValue("repro-satiety-threshold",0,1);
 c.herbivores.reproduction_well_fed_ticks=numberValue("repro-well-fed-ticks",1,10000);
 if(!Number.isInteger(c.herbivores.reproduction_well_fed_ticks))throw new Error("満腹継続Tickは整数で指定してください");
 if(c.herbivores.satiety_reproduction_enabled && !c.herbivores.satiety_enabled)throw new Error("満腹連動繁殖をONにする場合は満腹度システムもONにしてください");
 c.herbivores.resource_dependent_reproduction=(el("resource-reproduction") as HTMLSelectElement).value==="on";
 c.herbivores.reproduction_food_radius=numberValue("food-radius",1,500);
 c.herbivores.reproduction_food_target=numberValue("food-target",1,1000);
 if(!Number.isInteger(c.herbivores.reproduction_food_target))throw new Error("周辺植物数は整数で指定してください");
 c.herbivores.feeding_nutrients_per_tick=numberValue("feeding",0,10);
 c.initial_population.herbivores=numberValue("herbivores",0,5000);
 if(!Number.isInteger(c.initial_population.herbivores))throw new Error("初期草食動物数は整数で指定してください");
 const size=(el("field-size") as HTMLSelectElement).value;
 return resizeField(c,size==="current"?null:Number(size),(el("field-density") as HTMLSelectElement).value==="scale");
}
function downloadText(name:string,content:string,mime:string){
 const url=URL.createObjectURL(new Blob([content],{type:mime}));
 const a=document.createElement("a");a.href=url;a.download=name;document.body.append(a);a.click();a.remove();setTimeout(()=>URL.revokeObjectURL(url),1000);
}
function showConfigInControls(c:Config){
 (el("field-size") as HTMLSelectElement).value=c.world.land_columns===c.world.land_rows && [20,30,40].includes(c.world.land_columns)?String(c.world.land_columns):"current";
 (el("field-density") as HTMLSelectElement).value="scale";
 const values:Record<string,number>={growth:c.plants.growth_per_tick,uptake:c.plants.nutrient_uptake_max,seed:c.plants.seed_probability,mineralization:c.plants.mineralization_rate,diffusion:c.plants.soil_diffusion_rate,radius:c.plants.seed_dispersal_radius,cooldown:c.plants.seed_cooldown_ticks,feeding:c.herbivores.feeding_nutrients_per_tick,herbivores:c.initial_population.herbivores,"food-radius":c.herbivores.reproduction_food_radius,"food-target":c.herbivores.reproduction_food_target,"plant-protection-threshold":c.herbivores.plant_protection_threshold,"repro-satiety-threshold":c.herbivores.reproduction_satiety_threshold,"repro-well-fed-ticks":c.herbivores.reproduction_well_fed_ticks,"stomach-capacity":c.herbivores.stomach_capacity_n,"satiety-stop":c.herbivores.feeding_stop_fraction,"satiety-resume":c.herbivores.feeding_resume_fraction,digestion:c.herbivores.digestion_n_per_tick};
 for(const [id,value] of Object.entries(values))(el(id) as HTMLInputElement).value=String(value);
 for(const [id,on] of Object.entries({"plant-protection":c.herbivores.plant_protection_enabled,"satiety-enabled":c.herbivores.satiety_enabled,"satiety-reproduction":c.herbivores.satiety_reproduction_enabled,"resource-reproduction":c.herbivores.resource_dependent_reproduction}))(el(id) as HTMLSelectElement).value=on?"on":"off";
 (el("preset") as HTMLSelectElement).value="custom";
}
function installConfig(nextConfig:Config, condition=activeCondition){
 if (!Number.isSafeInteger(nextConfig.seed) || nextConfig.seed < 0)
   throw new Error("seedは0以上の安全な整数で指定してください");
 const next=new Simulation(JSON.stringify(nextConfig));
 const stage4 = condition.startsWith("stage4-");
 // The Stage 4 batch did not enable optional soil germination.

 const previous = simulation;
 try {
   if(nextConfig.toxin.enabled && !stage4)next.enable_standard_germination();
   const nextInterval = dev ? Number((el("log-interval") as HTMLInputElement).value) : 100;
   const nextStop = dev ? Number((el("stop-tick") as HTMLInputElement).value) : 0;
   if (!Number.isSafeInteger(nextInterval) || nextInterval < 1 || !Number.isSafeInteger(nextStop) || nextStop < 0)
     throw new Error("記録間隔・終了Tickを整数で指定してください");
   const recordLineage = dev && (el("lineage-enabled") as HTMLInputElement).checked;
   if (recordLineage && nextStop === 0)
     throw new Error("系譜記録には正の終了Tickが必要です");
   if (recordLineage) next.enable_lineage_observation(BigInt(nextStop));
   interval=nextInterval; stopAt=nextStop; lineage=recordLineage ? [] : null;
 } catch (e) { next.free(); throw e; }
 playing=false; el("play").textContent="再生";
 previous?.free(); simulation=next; activeConfig=nextConfig; activeCondition=condition;
 latestLineageSample=null; lineageFinished=false; populationTrend.length=0; priorTick=-1; mimicryBucket=-1; observedPresent=[false,false,false]; map.reset(); ecology.reset();
 if (dev) { resetLog(); settings.load(JSON.parse(simulation.effective_config_json())); }
 el("pending").textContent="編集中の設定は実行中の設定と一致しています";
 el("settings-error").textContent="";
 el("running-config").textContent=`実行中 · Seed ${nextConfig.seed} · 植物毒 ${nextConfig.toxin.enabled ? "ON" : "OFF"}`;
 el("phase19-active").textContent=playNames[condition] ?? condition;
 el("preset-note").textContent=playNotes[condition] ?? "";
 render();
 if(dev) status.textContent=`設定適用完了 — Seed ${nextConfig.seed} / 肉食動物初期 ${nextConfig.initial_population.carnivores} / 持続探索 ${nextConfig.carnivores.persistent_search_enabled}`;
}
function resetWithSettings(){
 try{
  const nextConfig=readConfig();
  installConfig(nextConfig);
  showConfigInControls(nextConfig);
 }catch(e){status.textContent=`設定エラー: ${formatSettingError(e)}`;}
}

let playing=false;
let last=0;
function render(){
  const renderStart = performance.now();
  el("condition-seed").textContent=String(activeConfig.seed);
  el("hungry-color-recheck").textContent=simulation.hungry_color_recheck_enabled()?"有効（予備E 25%未満）":"無効";
  el("toxin-mode").textContent=activeConfig.toxin.enabled ? `植物毒 ${activeConfig.toxin.hp_damage_per_excess_unit===0.01?"（1倍）":`（係数 ${activeConfig.toxin.hp_damage_per_excess_unit}）`}` : "植物毒OFF";
  const mean=simulation.resistance_mean();
  el("resistance-mean").textContent=Number.isFinite(mean)?mean.toFixed(4):"—（草食動物なし）";
  el("resistance-high").textContent=String(simulation.resistance_high_count());
  el("spontaneous-births").textContent=String(simulation.spontaneous_plant_births());
  el("tick").textContent=String(simulation.tick_count());
  if(dev){
  const predatorDebug=JSON.parse(simulation.carnivore_debug_json()) as Metrics;
  el("carnivores").textContent=String(simulation.carnivore_count());
  const genetics=JSON.parse(simulation.ecology_snapshot_json()) as Metrics;
  el("ambush-gene-mean").textContent=simulation.carnivore_count()?Number(genetics.carnivore_ambush_gene_mean??0).toFixed(3):"—（絶滅）";
  el("ambush-gene-var").textContent=simulation.carnivore_count()?Number(genetics.carnivore_ambush_gene_variance??0).toFixed(4):"—（絶滅）";
  el("carnivore-hunts").textContent=String(simulation.carnivore_hunts());
  el("carnivore-birth-death").textContent=`${simulation.carnivore_births()} / ${simulation.carnivore_deaths()}`;
  for(const [id,key] of [["carnivore-search","predator_search_ticks"],["carnivore-target","predator_target_ticks"],["carnivore-persistent","persistent_search_ticks"],["carnivore-turns","search_heading_changes"],["carnivore-gain","hunt_energy"],["carnivore-distance","predator_movement_distance"]]) el(id).textContent=String(predatorDebug[key]??0);
  el("animals").textContent=String(simulation.animal_count());
  const d=JSON.parse(simulation.diagnostics_json()) as Metrics;
  for(const [id,key] of [["p19-plant-births","plant_births"],["p19-plant-deaths","plant_age_deaths"],["p19-plant-eaten","plant_consumed_deaths"],["p19-herb-births","herbivore_births"],["p19-herb-hp-deaths","herbivore_hp_deaths"],["p19-herb-age-deaths","herbivore_age_deaths"],["p19-herb-zero-e","phase19_hp_zero_energy_deaths"],["p19-herb-positive-e","phase19_hp_positive_energy_deaths"]])el(id).textContent=String(d[key]??0);
  el("animal-energy").textContent=simulation.herbivore_energy().toFixed(2);
  el("plants").textContent=String(simulation.plant_count());
  el("soil").textContent=simulation.soil_nutrients().toFixed(2);
  el("detritus").textContent=simulation.detritus_nutrients().toFixed(2);
  el("nutrients").textContent=simulation.total_nutrients().toFixed(2);
  el("energy").textContent=simulation.total_energy().toFixed(2);
  el("births").textContent=String(simulation.births());
  el("deaths").textContent=String(simulation.deaths());
  }
  el("plants").textContent=String(simulation.plant_count());
  el("animals").textContent=String(simulation.herbivore_count());
  el("carnivores").textContent=String(simulation.carnivore_count());
  const {width,height,land_columns:columns,land_rows:rows}=activeConfig.world;
  el("field-description").textContent=`${columns} × ${rows} 土地セル / ${width} × ${height}`;
  map.draw({
    plant: JSON.parse(simulation.plant_color_snapshot_json()),
    herbivore: JSON.parse(simulation.herbivore_color_snapshot_json()),
    carnivore: JSON.parse(simulation.carnivore_snapshot_json()),
  }, simulation.map_entity_ids(), activeConfig.world,
    (index, generation) => simulation.inspect_entity_json(index, generation),
    JSON.parse(simulation.animal_shape_snapshot_json()));
  renderMs = performance.now() - renderStart;
  const tick = Number(simulation.tick_count());
  if (tick !== priorTick) {
    priorTick = tick;
    ecology.updateSpecies(simulation.species_summary_json());
    // Issue #72: mimicry indicators change at the 100-tick census; recompute only then.
    if (Math.floor(tick / 100) !== mimicryBucket) {
      mimicryBucket = Math.floor(tick / 100);
      drawMimicry(el("mimicry"), simulation.mimicry_json());
    }
    const counts = [simulation.plant_count(), simulation.herbivore_count(), simulation.carnivore_count()];
    counts.forEach((n, i) => { observedPresent[i] ||= n > 0; });
    el("extinction").textContent = counts.flatMap((n, i) => !n && observedPresent[i] ? [["植物","草食","肉食"][i] + "がいなくなりました"] : []).join("／");
    populationTrend.push(counts);
    if (populationTrend.length > 120) populationTrend.shift();
    const trend = el("population-trend") as HTMLCanvasElement;
    const g = trend.getContext("2d")!;
    g.clearRect(0,0,trend.width,trend.height);
    ["#83d990","#8cc7f0","#f7ad77"].forEach((color,i) => {
      g.strokeStyle=color;g.beginPath();
      const peak=Math.max(1,...populationTrend.map(row=>row[i]));
      populationTrend.forEach((row,j)=>{
        const x=j*trend.width/Math.max(1,populationTrend.length-1);
        const y=trend.height-4-row[i]/peak*(trend.height-8);
        if(j===0)g.moveTo(x,y);else g.lineTo(x,y);
      });g.stroke();
    });
    if (dev) el("research-summary").textContent = JSON.stringify({
      草食耐性分布: JSON.parse(simulation.living_resistance_distribution_json()),
      動物毒: JSON.parse(simulation.animal_toxin_snapshot_json()),
      系譜記録数: lineage?.length ?? "OFF",
      世代と生存: latestLineageSample ?? "系譜記録を有効にすると表示",
    }, null, 2);
  }
}
type Metrics=Record<string,number>;
const snapshotFields=new Set(["plants","herbivores","herbivore_energy","soil_n","detritus_n","total_n","total_e","nutrient_error","energy_error","soil_min_cell_n","soil_max_cell_n","soil_depleted_cells","soil_below_half_saturation_cells","plant_occupied_cells","plant_max_per_cell","plant_immature","plant_mature","plant_near_lifespan_end","plant_mean_growth","plant_body_n","plant_reserve_n","plant_reserve_e","herbivore_body_n","herbivore_reserve_n","herbivore_stomach_n","herbivore_satiety_mean","herbivore_satiety_min","herbivore_satiety_max"]);
const logRows:Record<string,string|number>[]=[];
let previous:Metrics|null=null;
let initialN=0,initialE=0;
function metrics():Metrics{
 const diag=JSON.parse(simulation.diagnostics_json()) as Metrics;
 const ecology=JSON.parse(simulation.ecology_snapshot_json()) as Metrics;
 const totalN=simulation.total_nutrients(), totalE=simulation.total_energy();
 return {tick:Number(simulation.tick_count()),carnivores:simulation.carnivore_count(),carnivore_hunts:Number(simulation.carnivore_hunts()),carnivore_births:Number(simulation.carnivore_births()),carnivore_deaths:Number(simulation.carnivore_deaths()),...(JSON.parse(simulation.carnivore_debug_json()) as Metrics),plants:simulation.plant_count(),herbivores:simulation.herbivore_count(),
  births:Number(simulation.births()),deaths:Number(simulation.deaths()),herbivore_energy:simulation.herbivore_energy(),
  soil_n:simulation.soil_nutrients(),detritus_n:simulation.detritus_nutrients(),total_n:totalN,total_e:totalE,
  sunlight_in:simulation.sunlight_in(),heat_out:simulation.heat_out(),
  nutrient_error:totalN-initialN,energy_error:totalE-initialE-simulation.sunlight_in()+simulation.heat_out(),
  ...diag,...ecology};
}
function resetLog(){logRows.length=0;previous=null;initialN=simulation.total_nutrients();initialE=simulation.total_energy();recordLog();}
function recordLog(){
 const m=metrics();const row:Record<string,string|number>={type:"snapshot",phase:"06.2.36",condition:(el("phase19-condition") as HTMLSelectElement).value||"custom",...m};
 for(const [key,value] of Object.entries(m)){
  if(key!=="tick" && previous && key in previous && !snapshotFields.has(key)){
   row[`delta_${key}`]=value-previous[key];
  }
 }
 logRows.push(row);previous=m;
 el("log-count").textContent=String(logRows.length);
}
function errorLog(message:string){
 logRows.push({type:"error",tick:Number(simulation.tick_count()),message});
 el("log-count").textContent=String(logRows.length);
}
function csvCell(v:string|number|undefined):string{
 const value=String(v??"");return `"${value.replaceAll('"','""')}"`;
}
function exportLog(){
 // A single wide CSV preserves all categories and 100-tick deltas, with error rows.
 const headers=[...new Set(logRows.flatMap(row=>Object.keys(row)))];
 const csv="\uFEFF"+[headers.map(csvCell).join(","),...logRows.map(row=>headers.map(h=>csvCell(row[h])).join(","))].join("\r\n");
 const url=URL.createObjectURL(new Blob([csv],{type:"text/csv;charset=utf-8"}));
 const a=document.createElement("a");a.href=url;a.download=`ecosystem-phase06236-${(el("phase19-condition") as HTMLSelectElement).value||"custom"}-seed${activeConfig.seed}-tick${simulation.tick_count()}.csv`;
 document.body.append(a);a.click();a.remove();setTimeout(()=>URL.revokeObjectURL(url),1000);
}
function step(n:number){
 const started = performance.now();
 try{
  const current = Number(simulation.tick_count());
  let left = stopAt ? Math.min(n, Math.max(0, stopAt - current)) : n;
  while(left>0){
   const toNext=dev ? interval-(Number(simulation.tick_count())%interval) : left;
   const batch=Math.min(left,toNext,activeConfig.simulation.max_steps_per_call);
   simulation.step(batch);left-=batch;
   ecology.addEvents(simulation.take_events_json());
   if (lineage) for(const event of JSON.parse(simulation.take_lineage_events_json())) {
     lineage.push(event);
     if (event.kind === "snapshot") latestLineageSample = event;
   }
   if(dev && Number(simulation.tick_count())%interval===0)recordLog();
  }
  if (stopAt && Number(simulation.tick_count()) >= stopAt) {
    playing = false; el("play").textContent = "再生";
    if (lineage && !lineageFinished) {
      simulation.finish_lineage_observation();
      for(const event of JSON.parse(simulation.take_lineage_events_json())) lineage.push(event);
      lineageFinished = true;
    }
  }
  render();
 }catch(e){playing=false;el("play").textContent="再生";const message=String(e);status.textContent=`実行エラー: ${message}`;if(dev)errorLog(message);render();}
 finally { stepMs = performance.now() - started; }
}
// ×1/×2/×4 denotes simulation ticks per 50 ms of wall time (not wall-clock acceleration guarantees).
let speedSample: { time: number; tick: number } | undefined;
function loop(t:number){
 if(dev){
   if(!frameStart) frameStart=t;
   frameCount++;
   if(t-frameStart>=1000){
     el("performance").textContent = `${(frameCount*1000/(t-frameStart)).toFixed(1)} FPS · マップ・画面更新 ${renderMs.toFixed(1)} ms · ステップ処理＋描画 ${stepMs.toFixed(1)} ms（直近）`;
     frameCount=0; frameStart=t;
   }
 }
 if(playing&&t-last>=50){
  const tick=Number(simulation.tick_count());
  if(!speedSample) speedSample={time:t,tick};
  else if(t-speedSample.time>=1000){
    const actual=(tick-speedSample.tick)*1000/(t-speedSample.time);
    el("actual-speed").textContent="実測 "+actual.toFixed(1)+" Tick/秒";
    speedSample={time:t,tick};
  }
  const speed=Number((el("play-speed") as HTMLSelectElement).value)||1;
  step(speed);last=t;
 }
 if(!playing&&speedSample){
  speedSample=undefined;
  el("actual-speed").textContent="停止中";
 }
 requestAnimationFrame(loop);
}
async function main(){
 try{
  await init();restoreInitialControls();showConfigInControls(baseline);
  const conditionSelect=el("phase19-condition") as HTMLSelectElement;
  const requested = new URLSearchParams(location.search).get("preset");
  if (!dev) {
    conditionSelect.replaceChildren(...Object.entries(playNames).map(([key, name])=>new Option(name,key)));
    if (requested && Object.hasOwn(phase19Configs, requested) && !Object.hasOwn(playNames, requested))
      conditionSelect.add(new Option("URL指定の環境", requested));
  }
  conditionSelect.value = requested && Object.hasOwn(phase19Configs, requested) ? requested : startupWebPreset;
  conditionSelect.onchange=()=>{
    try{
      const next=structuredClone(phase19Configs[conditionSelect.value]);
      installConfig(next,conditionSelect.value);
      baseline=structuredClone(next);
      showConfigInControls(next);
      const url = new URL(location.href);
      url.searchParams.set("preset",conditionSelect.value);
      history.replaceState(null,"",url);
      updateLinks();
    }catch(e){status.textContent=`条件変更エラー: ${String(e)}`;}
  };
  // 起動時はDOMのフォーム値を読まない。Firefoxのフォーム復元や空欄の影響を受けず、
  // バッチと同じ検証用JSONを直接Rust/WASMへ渡す。
  const updateLinks = () => {
    for (const mode of ["play", "dev"]) (el(mode+"-link") as HTMLAnchorElement).href = "/" + mode + location.search;
  };
  baseline = structuredClone(phase19Configs[conditionSelect.value]);
  installConfig(baseline, conditionSelect.value);
  showConfigInControls(baseline);
  updateLinks();
  for(const b of buttons)b.disabled=false;
  el("export-log").onclick=exportLog;
  el("export-config").onclick=()=>downloadText(`${activeCondition}-seed${activeConfig.seed}.json`,JSON.stringify({config:activeConfig,condition:activeCondition,observation:{interval,stopAt,lineage:lineage!==null}},null,2),"application/json;charset=utf-8");
  el("import-config").onclick=()=>{(el("config-file") as HTMLInputElement).click();};
  (el("config-file") as HTMLInputElement).onchange=async()=>{
   try{
    const file=(el("config-file") as HTMLInputElement).files?.[0];if(!file)return;
    const saved=JSON.parse(await file.text());
    const next=normalizeConfig(saved?.config ?? saved);
    const condition=typeof saved?.condition==="string"?saved.condition:
      (next as Config & {toxin:{experimental_inherited_plant_colors?: boolean}}).toxin.experimental_inherited_plant_colors
        ? "stage4-custom" : "custom";
    const observation = saved?.config ? saved.observation : undefined;
    const fields = ["log-interval","stop-tick","lineage-enabled"].map(id=>el(id) as HTMLInputElement);
    const previous = [fields[0].value,fields[1].value,fields[2].checked] as const;
    if (observation) {
      fields[0].value=String(observation.interval);
      fields[1].value=String(observation.stopAt);
      fields[2].checked=observation.lineage===true;
    }
    try { installConfig(next,condition); }
    catch (e) {
      [fields[0].value,fields[1].value,fields[2].checked]=previous;
      throw e;
    }
    baseline=structuredClone(next);showConfigInControls(next);conditionSelect.value="";
   }catch(e){status.textContent=`JSON読み込みエラー: ${formatSettingError(e)}`;}
   (el("config-file") as HTMLInputElement).value="";
  };
  el("step").onclick=()=>step(1);
  el("step100").onclick=()=>step(100);
  el("step500").onclick=()=>step(500);
  el("step1000").onclick=()=>step(1000);
  el("play").onclick=()=>{if(stopAt && Number(simulation.tick_count())>=stopAt)return; playing=!playing;el("play").textContent=playing?"一時停止":"再生";};
  // Play mode: each reset starts a new world with a fresh seed (dev keeps the seed for comparisons).
  el("reset").onclick=()=>{try {
      const next = dev ? activeConfig : { ...structuredClone(activeConfig), seed: Math.floor(Math.random() * 2 ** 32) };
      installConfig(next); showConfigInControls(next); }
    catch(e){status.textContent=String(e);}};
  el("apply").onclick=()=>resetWithSettings();
  if (dev) describeExistingControls();
  settings.onApply = (value, changed) => {
    try { const next=changed ? normalizeConfig(value) : structuredClone(activeConfig); installConfig(next); baseline=structuredClone(next); showConfigInControls(next); }
    catch(e){
      el("pending").textContent="適用できませんでした。編集値は保持されています。";
      el("settings-error").textContent=formatSettingError(e);
      status.textContent="設定を適用できませんでした。実行中の環境は操作できます。";
    }
  };
  el("capture-reference").onclick=()=>{
    reference={config:structuredClone(activeConfig),condition:activeCondition};
    el("reference-info").textContent="対照を保存 · Seed "+reference.config.seed+" · Tick "+simulation.tick_count()+
      " · 植物／草食／肉食 "+[simulation.plant_count(),simulation.herbivore_count(),simulation.carnivore_count()].join(" / ");
  };
  el("restore-reference").onclick=()=>{
    if(!reference)return;
    try{installConfig(structuredClone(reference.config),reference.condition);baseline=structuredClone(activeConfig);showConfigInControls(activeConfig);}
    catch(e){status.textContent=String(e);}
  };
  el("export-lineage").onclick=()=>{
    if(!lineage){status.textContent="系譜を記録するには終了Tickを指定して新規開始してください。";return;}
    const events=lineage.concat(JSON.parse(simulation.take_lineage_events_json()));
    lineage=events;
    downloadText("lineage-"+activeConfig.seed+".json",JSON.stringify({complete:lineageFinished,events}),"application/json");
  };
  for(const id of ["log-interval","stop-tick","lineage-enabled"]) el(id).oninput=()=>el("pending").textContent="観測設定に未適用の変更があります";
  el("compare").onclick=()=>{setPreset("baseline");resetWithSettings();};
  (el("preset") as HTMLSelectElement).onchange=()=>{const p=(el("preset") as HTMLSelectElement).value;if(p==="baseline"||p==="balanced"||p==="circulation")setPreset(p);};
  for(const id of ["field-size","field-density","growth","uptake","seed","mineralization","diffusion","radius","cooldown","feeding","herbivores","food-radius","food-target","plant-protection","plant-protection-threshold","resource-reproduction","satiety-enabled","satiety-reproduction","repro-satiety-threshold","repro-well-fed-ticks","stomach-capacity","satiety-stop","satiety-resume","digestion"])(el(id) as HTMLInputElement).oninput=()=>{(el("preset") as HTMLSelectElement).value="custom";el("pending").textContent="未適用の変更があります";};
  if (!dev) status.textContent="";

  // Issue #64: legend samples drawn with the same gene-based creature sprites as the map.
  const samples: Record<string, ["herbivore" | "carnivore", number[], [number, number, number]]> = {
    plain: ["herbivore", [60, 2.5, 0.2, 0.5], [0.3, 0.75, 0.4]],
    carnivore: ["carnivore", [0, 5.25, 0.2, -1], [0.3, 0.75, 0.4]],
    toxic: ["herbivore", [60, 2.5, 0.95, 0.5], [0.3, 0.75, 0.4]],
    fast: ["herbivore", [60, 4.5, 0.2, 0.5], [0.3, 0.75, 0.4]],
    shell: ["herbivore", [60, 2.5, 0.2, 0.65], [0.3, 0.75, 0.4]],
  };
  for (const canvas of document.querySelectorAll<HTMLCanvasElement>("canvas[data-legend]")) {
    const [kind, traits, rgb] = samples[canvas.dataset.legend!];
    drawCreature(canvas.getContext("2d")!, kind, 20, 20, 36, rgb, creatureShape(kind, traits));
  }
  render();requestAnimationFrame(loop);
 }catch(e){status.textContent=`初期化失敗: ${String(e)}`;}
}
void main();
