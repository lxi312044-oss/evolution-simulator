import {readFileSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const [input,output]=process.argv.slice(2);
const hash=p=>createHash('sha256').update(readFileSync(p)).digest('hex');
const runtime=process.argv[5]??'web/src/pkg';
const js=resolve(runtime,'sim_wasm.js'),binary=resolve(runtime,'sim_wasm_bg.wasm');
const {initSync,Simulation}=await import(pathToFileURL(js).href);initSync({module:readFileSync(binary)});
const config=JSON.parse(readFileSync(input,'utf8'));const s=new Simulation(JSON.stringify(config));s.enable_standard_germination();
const initialN=s.total_nutrients(),initialE=s.total_energy();
const initialPublic={plants:JSON.parse(s.plant_snapshot_json()),animals:JSON.parse(s.herbivore_color_snapshot_json()),carnivores:JSON.parse(s.carnivore_snapshot_json()),n:initialN,e:initialE,soil:s.soil_nutrients(),detritus:s.detritus_nutrients()};
const samples=[];const start=performance.now();const ticks=Number(process.argv[4]??50000);
for(let tick=0;tick<=ticks;tick+=1000){
 if(tick)s.step(1000);
 const present=s.animal_count(),diag=JSON.parse(s.diagnostics_json());
 for(const k of ['herbivore_births','herbivore_hp_deaths','herbivore_age_deaths','toxin_exposure_total','toxin_hp_damage_total','resistance_energy_cost_total','resistance_mutations','herbivore_plant_protection_blocked_ticks','phase21_starvation_damage_ticks'])if(!(k in diag))throw Error('Missing diagnostic '+k);
 samples.push({tick:Number(s.tick_count()),present,positive:JSON.parse(s.herbivore_color_snapshot_json()).filter(a=>a[3]>0).length,plants:s.plant_count(),carnivores:s.carnivore_count(),resistance_mean:present?s.resistance_mean():null,high:s.resistance_high_count(),high_fraction:present?s.resistance_high_count()/present:null,germination_births:s.spontaneous_plant_births(),births:diag.herbivore_births,hp_deaths:diag.herbivore_hp_deaths,age_deaths:diag.herbivore_age_deaths,dose:diag.toxin_exposure_total,damage:diag.toxin_hp_damage_total,upkeep:diag.resistance_energy_cost_total,mutations:diag.resistance_mutations,protected_ticks:diag.herbivore_plant_protection_blocked_ticks,starvation_ticks:diag.phase21_starvation_damage_ticks,total_n:s.total_nutrients(),total_e:s.total_energy(),sunlight_in:s.sunlight_in(),heat_out:s.heat_out(),n_error:s.total_nutrients()-initialN,e_error:s.total_energy()-initialE-s.sunlight_in()+s.heat_out()});
 if(performance.now()-start>600000)throw Error('10-minute cap');
}
s.free();writeFileSync(output,JSON.stringify({ticks,config,initial_n:initialN,initial_e:initialE,input_sha256:hash(input),wasm_sha256:hash(binary),js_sha256:hash(js),initial_public_sha256:createHash('sha256').update(JSON.stringify(initialPublic)).digest('hex'),elapsed_seconds:(performance.now()-start)/1000,samples}));
