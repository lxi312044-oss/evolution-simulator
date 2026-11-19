import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import assert from 'node:assert/strict';
const [input,runtime,valueArg,ticksArg,shadowArg='on']=process.argv.slice(2);
const value=Number(valueArg),ticks=Number(ticksArg);
const {initSync,Simulation}=await import(pathToFileURL(resolve(runtime,'sim_wasm.js')).href);
initSync({module:readFileSync(resolve(runtime,'sim_wasm_bg.wasm'))});
const config=readFileSync(input,'utf8');
function make(){const s=new Simulation(config);s.enable_standard_germination();return s;}
const s=make(),id=JSON.parse(s.intervention_candidates_json())[0];let shadow=make();
assert(id,'no focal');const before=JSON.parse(s.observation_state_json());
assert.deepEqual(before,JSON.parse(shadow.observation_state_json()));
// Invalid calls must leave the world unchanged and must not consume the one-shot intervention.
for(const [i,g,v] of [[...id,NaN],[...id,Infinity],[...id,-.1],[...id,1.1],[4294967295,0,.2],[id[0],id[1]+1,.2]]){
 assert.throws(()=>s.intervene_focal_resistance(i,g,v));assert.deepEqual(before,JSON.parse(s.observation_state_json()));
}
const receipt=JSON.parse(s.intervene_focal_resistance(...id,value));
assert.deepEqual(receipt,JSON.parse(shadow.intervene_focal_resistance(...id,value)));
const after=JSON.parse(s.observation_state_json());
const expected={...before,animals:before.animals.replace(receipt.before,receipt.after)};
assert.notEqual(expected.animals,before.animals);
assert.deepEqual(after,expected);
assert.equal(receipt.after,receipt.before.replace('toxin_resistance: '+receipt.old+',','toxin_resistance: '+(value===.2?'0.2':'0.8')+','));
assert.throws(()=>s.intervene_focal_resistance(...id,value));
const late=make();late.step(1);assert.throws(()=>late.intervene_focal_resistance(...id,value));late.free();
const observed=make();observed.enable_lineage_observation(1000n);assert.throws(()=>observed.intervene_focal_resistance(...id,value));observed.free();
const multi=JSON.parse(config);multi.toxin.herbivore_multi_toxin_enabled=true;
const m=new Simulation(JSON.stringify(multi));assert.throws(()=>m.intervene_focal_resistance(...id,value));m.free();
const emit=r=>process.stdout.write(JSON.stringify(r)+'\n');
emit({kind:'intervention_audit',before,receipt,invalid_calls_rejected:true});
if(shadowArg==='off'){shadow.free();shadow=null;}
s.enable_lineage_observation(BigInt(ticks));
function drain(){for(const r of JSON.parse(s.take_lineage_events_json()))emit(r);}
if(shadow)assert.equal(s.observation_state_json(),shadow.observation_state_json());drain();
for(let t=1000;t<=ticks;t+=1000){s.step(1000);if(shadow){shadow.step(1000);assert.equal(s.observation_state_json(),shadow.observation_state_json());}drain();}
s.finish_lineage_observation();drain();
emit({kind:'verification',shadow_equal:shadow?true:null,checkpoints:shadow?ticks/1000+1:0});
s.free();if(shadow)shadow.free();
