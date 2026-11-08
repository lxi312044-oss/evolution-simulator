import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
const [input,runtime,ticksArg='50000',shadowArg='off']=process.argv.slice(2);const ticks=Number(ticksArg);
const {initSync,Simulation}=await import(pathToFileURL(resolve(runtime,'sim_wasm.js')).href);initSync({module:readFileSync(resolve(runtime,'sim_wasm_bg.wasm'))});
const config=readFileSync(input,'utf8');const s=new Simulation(config);s.enable_standard_germination();
const shadow=shadowArg==='on'?new Simulation(config):null;if(shadow)shadow.enable_standard_germination();
s.enable_lineage_observation(BigInt(ticks));
let repeatedEnableRejected=false;try{s.enable_lineage_observation(BigInt(ticks));}catch{repeatedEnableRejected=true;}if(!repeatedEnableRejected)throw Error('duplicate enable accepted');
const n0=s.total_nutrients(),e0=s.total_energy();
function emit(row){process.stdout.write(JSON.stringify(row)+'\n');}
function drain(){for(const row of JSON.parse(s.take_lineage_events_json()))emit(row);}
function publicSample(){
 const d=JSON.parse(s.diagnostics_json()),n=s.animal_count();
 return {tick:Number(s.tick_count()),present:n,positive:JSON.parse(s.herbivore_color_snapshot_json()).filter(a=>a[3]>0).length,plants:s.plant_count(),carnivores:s.carnivore_count(),resistance_mean:n?s.resistance_mean():null,high:s.resistance_high_count(),high_fraction:n?s.resistance_high_count()/n:null,germination_births:s.spontaneous_plant_births(),births:d.herbivore_births,hp_deaths:d.herbivore_hp_deaths,age_deaths:d.herbivore_age_deaths,dose:d.toxin_exposure_total,damage:d.toxin_hp_damage_total,upkeep:d.resistance_energy_cost_total,mutations:d.resistance_mutations,protected_ticks:d.herbivore_plant_protection_blocked_ticks,starvation_ticks:d.phase21_starvation_damage_ticks,total_n:s.total_nutrients(),total_e:s.total_energy(),sunlight_in:s.sunlight_in(),heat_out:s.heat_out(),n_error:s.total_nutrients()-n0,e_error:s.total_energy()-e0-s.sunlight_in()+s.heat_out()};
}
function compareShadow(){if(shadow&&s.observation_state_json()!==shadow.observation_state_json())throw Error('observation changed full state/RNG at '+s.tick_count());}
compareShadow();drain();emit({kind:'public',sample:publicSample()});
for(let tick=1000;tick<=ticks;tick+=1000){s.step(1000);if(shadow)shadow.step(1000);compareShadow();drain();emit({kind:'public',sample:publicSample()});}
s.finish_lineage_observation();drain();if(JSON.parse(s.take_lineage_events_json()).length!==0)throw Error('events not drained');
let duplicateFinishRejected=false;try{s.finish_lineage_observation();}catch{duplicateFinishRejected=true;}if(!duplicateFinishRejected)throw Error('duplicate finish accepted');
if(shadow){emit({kind:'verification',shadow_equal:true,checkpoints:ticks/1000+1});shadow.free();}s.free();
