import fs from "node:fs";
import { gzipSync } from "node:zlib";
import {createHash} from "node:crypto";
import assert from "node:assert/strict";
import {createRequire} from "node:module";
import {resolve} from "node:path";
const dir=resolve("target/issue11-baseline-35400");
const [threshold, mode, reserveFloor]=process.argv.slice(2);
const reserveRun = reserveFloor !== undefined;
const read=n=>fs.readFileSync(resolve(dir,n));
const sha=b=>createHash("sha256").update(b).digest("hex");
assert.equal(sha(read("input.json")),"91ccf52ed4b3285380aaf041851f792fd198878e84022a48ec5c66f8dee34b41");
assert.equal(sha(read("pkg/sim_wasm_bg.wasm")),"19662a00b4f2dd8fe96fcd1195dda7c46206d6d91b3fddfcfc496e817a17f9d0");
assert.equal(sha(read("individual.json")),"2fb578cf3f1c0791aee674743df1f472ce0c24074882a87fbcc6bb293ddcf17d");
const wasmDir=reserveRun?resolve(dir,"../issue11-parent-reserve/pkg"):resolve(dir,"pkg");
const wasmHash=sha(fs.readFileSync(resolve(wasmDir,"sim_wasm_bg.wasm")));
const {Simulation}=createRequire(import.meta.url)(resolve(wasmDir,"sim_wasm.js"));
assert.ok(["0.25","0.40"].includes(threshold));
assert.ok(["observe","audit","control"].includes(mode));
const original=JSON.parse(read("input.json")).config;
const config=structuredClone(original);
assert.equal(config.carnivores.reproduction_stomach_max_fraction,.25);
config.carnivores.reproduction_stomach_max_fraction=Number(threshold);
if(reserveRun){
 assert.equal(threshold,"0.40");
 assert.ok(["0","4"].includes(reserveFloor));
 config.carnivores.reproduction_parent_reserve_energy=Number(reserveFloor);
}
const sim=new Simulation(JSON.stringify(config));
sim.enable_standard_germination();
const start=performance.now();
const hashes=[], rows=[], records={}, births=[];
const stem = reserveRun ? "reserve-"+reserveFloor+"-"+mode : threshold+"-"+mode;
const detailPath = resolve(dir, stem+".ndjson.gz");
const manifestPath = resolve(dir, stem+"-status.json");
const existing = fs.readdirSync(dir).filter(n=>/\.(json|gz)$/.test(n))
 .reduce((sum,n)=>sum+fs.statSync(resolve(dir,n)).size,0);
const budget = 32*1024*1024;
const reserve = 2*1024*1024;
const fixed = {threshold,mode,config,wasm:wasmHash,input:sha(read("input.json"))};
let used = existing, savedTick = null, reason = "interrupted";
assert.ok(used+reserve < budget);
const output = fs.openSync(detailPath,"wx");
const status = () => fs.writeFileSync(manifestPath,JSON.stringify({...fixed,savedTick,reason,seconds:(performance.now()-start)/1000}));
status();
let deaths=[], max=0, firstCandidate=null, firstBirthDifference=null;
const reference=reserveRun?JSON.parse(read("0.40-observe.json")):null;
const referenceBirths=new Map();
for(const birth of reference?.births??[]){
 const list=referenceBirths.get(birth.tick)??[];
 list.push(birth);
 referenceBirths.set(birth.tick,list);
}
const cap=5000;
const limit=mode==="observe"?230000:240000;
try {
for(let t=0;t<=35400;t+=100){
 if(performance.now()-start>=limit-1000){reason="time";break;}
 const details=[], decisionRows=[];
 const birthStart=births.length, deathStart=deaths.length;
 if(t&&mode==="control") sim.step(100);
 if(t&&mode!=="control"){
  const b=JSON.parse(sim.carnivore_baseline_batch(100));
  if(mode==="audit" && firstCandidate===null){
   firstCandidate=b.events.find(e=>e.phase==="predator_reproduction"&&e.stomach===false&&e.fullness<=0.40&&e.food!==false&&e.trend!==false&&e.sustainable!==false)??null;
  }
  if(mode==="observe"){
   const latest = new Map();
   if(reference && !firstCandidate){
    firstCandidate=b.events.find(e=>e.phase==="predator_reproduction"&&e.reserve_e>=24&&e.reserve_e<28
     &&e.fail.every((failed,i)=>i===5||!failed))??null;
   }
   for(const e of b.events){
    if(e.phase!=="predator_reproduction")continue;
    const id=e.id.join(":");
    latest.set(id,e);
    if(reference && !firstBirthDifference && (referenceBirths.has(e.tick)||e.tick%100===0))
     decisionRows.push(e);
    if(!records[id]){
     records[id]={parent:e.parent,birth:e.parent?e.tick:0,first:null,ticks:0,stomachOnly:0,recent:[]};
     if(e.parent){
      const parent=e.parent.join(":");
      births.push({tick:e.tick,id,parent,parentGate:latest.get(parent)??null});
     }
    }
    const r=records[id];
    r.ticks++;
    if(r.first===null&&(e.bite_e>0||e.bite_n>0)) r.first=e.tick;
    if(e.stomach===false&&e.food!==false&&e.trend!==false&&e.sustainable!==false)r.stomachOnly++;
    assert.ok(e.population<=cap);
    max=Math.max(max,e.population);
    r.recent[e.tick%201]=e;
    if(e.tick<=r.birth+200)details.push(e);
   }
   for(const d of b.deaths.slice(deaths.length)){
    const r=records[d.id_index+":"+d.id_generation];
    details.push(...r.recent.filter(e=>e.tick>r.birth+200&&e.tick>=d.tick-200));
    delete r.recent;
   }
   deaths=b.deaths;
   if(reference && !firstBirthDifference){
    for(let tick=Math.max(1,t-99);tick<=t;tick++){
     const ref=referenceBirths.get(tick)??[];
     const now=births.slice(birthStart).filter(x=>x.tick===tick);
     if(JSON.stringify(ref.map(x=>[x.id,x.parent]))!==JSON.stringify(now.map(x=>[x.id,x.parent]))){
      firstBirthDifference={tick,ref,now,candidate:decisionRows.filter(e=>e.tick===tick)};
      break;
     }
    }
   }
  }
 }
 hashes.push(sha(sim.audit_state(false)));
 const counts=[sim.plant_count(),sim.herbivore_count(),sim.carnivore_count()];
 assert.ok(counts.reduce((a,b)=>a+b)<=cap);
 rows.push([t,...counts,Number(sim.carnivore_births()),Number(sim.carnivore_deaths()),Number(sim.carnivore_hunts())]);
 const chunk=gzipSync(JSON.stringify({tick:t,hash:hashes.at(-1),row:rows.at(-1),
  births:births.slice(birthStart), deaths:deaths.slice(deathStart),details,
  records:mode==="observe"?Object.fromEntries(Object.entries(records).map(([k,{recent,...r}])=>[k,r])):undefined,
  firstCandidate,firstBirthDifference})+"\n",{level:1});
 if(used+chunk.length+reserve>budget){reason="capacity";break;}
 if(performance.now()-start>=limit-500){reason="time";break;}
 fs.writeFileSync(output,chunk);
 used+=chunk.length;
 savedTick=t;
 status();
 if(t%5000===0) console.log(threshold,mode,t,Math.round((performance.now()-start)/1000));
}
if(savedTick===35400){
 for(const r of Object.values(records)) delete r.recent;
 const debug=JSON.parse(sim.carnivore_debug_json());
 const payload=Buffer.from(JSON.stringify({threshold,mode,firstCandidate,firstBirthDifference,hashes,rows,records,births,deaths,debug,max,seconds:(performance.now()-start)/1000}));
 if(payload.length+16384>reserve){reason="capacity";}
 else if(performance.now()-start>=limit-250){reason="time";}
 else {
  fs.writeFileSync(resolve(dir,stem+".json"),payload,{flag:"wx"});
  reason="complete";
 }
}
} catch(error) {
 reason=String(error);
 throw error;
} finally {
 fs.closeSync(output);
 status();
 console.log(threshold,mode,reason,savedTick,used-existing);
}
if(reason!=="complete") process.exitCode=1;
