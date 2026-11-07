import init, { Simulation } from "./pkg/sim_wasm";
import config from "../../config/standard-toxin.json";
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
import {drawAnimalSprite,drawPlantSprite} from "./sprites";
const el=(id:string)=>document.getElementById(id)!;
const status=el("status");
const canvas=el("world") as HTMLCanvasElement;
const ctx=canvas.getContext("2d")!;
const buttons=[el("step"),el("step100"),el("step500"),el("step1000"),el("play"),el("reset"),el("apply"),el("compare"),el("export-log"),el("export-config"),el("import-config")] as HTMLButtonElement[];
type Config = typeof config;
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
function installConfig(nextConfig:Config){
 const next=new Simulation(JSON.stringify(nextConfig));
 if(nextConfig.toxin.enabled)next.enable_standard_germination();
 playing=false;el("play").textContent="再生";
 simulation.free();simulation=next;activeConfig=nextConfig;resetLog();render();
 status.textContent=`設定適用完了 — Seed ${nextConfig.seed} / 肉食動物初期 ${nextConfig.initial_population.carnivores} / 持続探索 ${nextConfig.carnivores.persistent_search_enabled}`;
}
function resetWithSettings(){
 try{
  const nextConfig=readConfig();
  installConfig(nextConfig);
 }catch(e){status.textContent=`設定エラー: ${String(e)}`;}
}

let playing=false;
let last=0;
function render(){
  el("toxin-mode").textContent=activeConfig.toxin.enabled ? `通常毒 ${activeConfig.toxin.hp_damage_per_excess_unit===0.01?"（1倍）":`（係数 ${activeConfig.toxin.hp_damage_per_excess_unit}）`}` : "毒なし";
  const mean=simulation.resistance_mean();
  el("resistance-mean").textContent=Number.isFinite(mean)?mean.toFixed(4):"—（草食動物なし）";
  el("resistance-high").textContent=String(simulation.resistance_high_count());
  el("spontaneous-births").textContent=String(simulation.spontaneous_plant_births());
  el("tick").textContent=String(simulation.tick_count());
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
  const plants=JSON.parse(simulation.plant_snapshot_json()) as [number,number,number,number][];
  ctx.fillStyle="#0d1e20";ctx.fillRect(0,0,canvas.width,canvas.height);
  const {width,height,land_columns:columns,land_rows:rows}=activeConfig.world;
  const sx=canvas.width/width,sy=canvas.height/height;
  el("field-description").textContent=`${columns} × ${rows} 土地セル / ${width} × ${height}`;
  ctx.strokeStyle="rgba(155,190,165,.12)";ctx.lineWidth=1;
  for(let i=0;i<=columns;i++){const x=i*canvas.width/columns;ctx.beginPath();ctx.moveTo(x,0);ctx.lineTo(x,canvas.height);ctx.stroke();}
  for(let i=0;i<=rows;i++){const y=i*canvas.height/rows;ctx.beginPath();ctx.moveTo(0,y);ctx.lineTo(canvas.width,y);ctx.stroke();}
  for(const [x,y,r,g] of plants){drawPlantSprite(ctx,x*sx,y*sy,g,Math.max(7,r*Math.min(sx,sy)*2));}
  const predators=JSON.parse(simulation.carnivore_snapshot_json()) as [number,number,number,number,number,number,number][];
  for(const [x,y,r,_hp,red,green,blue] of predators){
    drawAnimalSprite(ctx,"carnivore",x*sx,y*sy,Math.max(16,r*Math.min(sx,sy)*2),false,[red,green,blue]);
  }
  const animals=JSON.parse(simulation.herbivore_color_snapshot_json()) as [number,number,number,number,number,number,number][];
  for(const [x,y,r,hp,red,green,blue] of animals){
    const px=x*sx,py=y*sy;
    drawAnimalSprite(ctx,"herbivore",px,py,Math.max(14,r*Math.min(sx,sy)*2),false,[red,green,blue]);
    if(hp<0.3){ctx.fillStyle="#f46b6b";ctx.fillRect(Math.round(px)-3,Math.round(py)-13,6,2);}
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
 try{
  let left=n;
  while(left>0){
   const toNext=100-(Number(simulation.tick_count())%100);
   const batch=Math.min(left,toNext);
   simulation.step(batch);left-=batch;
   if(Number(simulation.tick_count())%100===0)recordLog();
  }
  render();
 }catch(e){playing=false;el("play").textContent="再生";const message=String(e);status.textContent=`実行エラー: ${message}`;errorLog(message);render();}
}
// ×1/×2/×4 denotes simulation ticks per 50 ms of wall time (not wall-clock acceleration guarantees).
function loop(t:number){
 if(playing&&t-last>=50){
  const speed=Number((el("play-speed") as HTMLSelectElement).value)||1;
  step(speed);last=t;
 }
 requestAnimationFrame(loop);
}
async function main(){
 try{
  await init();restoreInitialControls();showConfigInControls(baseline);
  const conditionSelect=el("phase19-condition") as HTMLSelectElement;
  conditionSelect.value="standard-toxin";
  conditionSelect.onchange=()=>{
    try{
      const next=structuredClone(phase19Configs[conditionSelect.value]);
      baseline=structuredClone(next);
      showConfigInControls(next);
      installConfig(next);
      el("phase19-active").textContent=conditionSelect.selectedOptions[0]?.textContent??conditionSelect.value;
    }catch(e){status.textContent=`条件変更エラー: ${String(e)}`;}
  };
  // 起動時はDOMのフォーム値を読まない。Firefoxのフォーム復元や空欄の影響を受けず、
  // バッチと同じ検証用JSONを直接Rust/WASMへ渡す。
  activeConfig=structuredClone(baseline);simulation=new Simulation(JSON.stringify(activeConfig));
  if(activeConfig.toxin.enabled)simulation.enable_standard_germination();
  for(const b of buttons)b.disabled=false;
  resetLog();
  el("phase19-active").textContent=conditionSelect.selectedOptions[0]?.textContent??"遺伝型";
  el("export-log").onclick=exportLog;
  el("export-config").onclick=()=>downloadText(`standard-toxin-seed${activeConfig.seed}.json`,JSON.stringify(activeConfig,null,2),"application/json;charset=utf-8");
  el("import-config").onclick=()=>{(el("config-file") as HTMLInputElement).click();};
  (el("config-file") as HTMLInputElement).onchange=async()=>{
   try{
    const file=(el("config-file") as HTMLInputElement).files?.[0];if(!file)return;
    const next=normalizeConfig(JSON.parse(await file.text()));
    installConfig(next);baseline=structuredClone(next);showConfigInControls(next);conditionSelect.value="";el("phase19-active").textContent="外部JSON（カスタム）";
   }catch(e){status.textContent=`JSON読み込みエラー: ${String(e)}`;}
   (el("config-file") as HTMLInputElement).value="";
  };
  el("step").onclick=()=>step(1);
  el("step100").onclick=()=>step(100);
  el("step500").onclick=()=>step(500);
  el("step1000").onclick=()=>step(1000);
  el("play").onclick=()=>{playing=!playing;el("play").textContent=playing?"一時停止":"再生";};
  el("reset").onclick=()=>resetWithSettings();
  el("apply").onclick=()=>resetWithSettings();
  el("compare").onclick=()=>{setPreset("baseline");resetWithSettings();};
  (el("preset") as HTMLSelectElement).onchange=()=>{const p=(el("preset") as HTMLSelectElement).value;if(p==="baseline"||p==="balanced"||p==="circulation")setPreset(p);};
  for(const id of ["field-size","field-density","growth","uptake","seed","mineralization","diffusion","radius","cooldown","feeding","herbivores","food-radius","food-target","plant-protection","plant-protection-threshold","resource-reproduction","satiety-enabled","satiety-reproduction","repro-satiety-threshold","repro-well-fed-ticks","stomach-capacity","satiety-stop","satiety-resume","digestion"])(el(id) as HTMLInputElement).oninput=()=>{(el("preset") as HTMLSelectElement).value="custom";};
  status.textContent=`WASM初期化成功 — Seed ${activeConfig.seed} / 肉食動物初期 ${activeConfig.initial_population.carnivores} / 検証設定`;
  // The preview and the world both render live carnivore sprites.
  const preview=el("carnivore-preview") as HTMLCanvasElement;
  const previewCtx=preview.getContext("2d")!;
  drawAnimalSprite(previewCtx,"carnivore",24,24,40);
  render();requestAnimationFrame(loop);
 }catch(e){status.textContent=`初期化失敗: ${String(e)}`;}
}
void main();
