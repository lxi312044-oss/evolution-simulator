"""Fixed Issue 35 causal focal-resistance comparison on the qualified WASM."""
import argparse,csv,gzip,json,subprocess,time,shutil
from concurrent.futures import ThreadPoolExecutor,as_completed
from pathlib import Path
from run_longterm_resistance import ROOT,config_for,SEEDS,interval
from run_confirmation import save,require,now,software,archive_source,provenance,bytes_under
from run_wasm_lineage import FitnessAudit
from run_metadata import sha256
from verify_focal_intervention import audit as audit_genealogies
WASM='006a56928860ff2422d1ffcdebd03ef37ab148dd3d71e64fc94be17675e3b8ff'
WORKER=ROOT/'tools/focal_intervention_worker.mjs'

def assess(rows):
 indexed={(s['seed'],s['toxic'],s['value']):s for s in rows}
 require(len(rows)==32 and set(indexed)=={(seed,t,v) for seed in SEEDS for t in (False,True) for v in (.2,.8)},'matrix mismatch')
 details=[];contrasts=[]
 for metric in ('focal_direct_births','focal_descendant_births'):
  for seed in SEEDS:
   d={t:indexed[(seed,t,.8)][metric]-indexed[(seed,t,.2)][metric] for t in (False,True)}
   details.append(dict(seed=seed,metric=metric,toxin_off=d[False],toxin_on=d[True],interaction=d[True]-d[False]))
  sub=[d for d in details if d['metric']==metric]
  for effect in ('toxin_off','toxin_on','interaction'):
   contrasts.append(dict(metric=metric,effect=effect,**interval([d[effect] for d in sub])))
 return dict(seed_differences=details,contrasts=contrasts)

def replay(case,out,runtime,start,resume):
 seed,toxic,value=case;name=f'seed{seed}_toxin{int(toxic)}_r{value}'
 c=config_for(seed,40,toxic);save(out/(name+'.input.json'),c)
 if resume is not None and (resume/(name+'.summary.json')).exists():
  require(json.loads((resume/(name+'.input.json')).read_text(encoding='utf-8'))==c,'resume config mismatch')
  for suffix in ('.summary.json','.jsonl.gz','.stderr.log'):shutil.copyfile(resume/(name+suffix),out/(name+suffix))
  s=json.loads((out/(name+'.summary.json')).read_text(encoding='utf-8'))
  require((s['seed'],s['toxic'],s['value'],s['ticks'])==(seed,toxic,value,50000),'resume summary protocol mismatch')
  require(s['shadow_equal'] is (True if seed==SEEDS[0] else None),'resume shadow allocation mismatch')
  return s
 require(seed!=SEEDS[0],'qualified long shadow source required')
 remaining=1800-(time.monotonic()-start);require(remaining>0,'30-minute cap')
 proc=subprocess.run(['node',str(WORKER),str(out/(name+'.input.json')),str(runtime),str(value),'50000','off'],capture_output=True,text=True,encoding='utf-8',timeout=min(600,remaining))
 (out/(name+'.stderr.log')).write_text(proc.stderr,encoding='utf-8')
 with gzip.open(out/(name+'.jsonl.gz'),'wt',encoding='utf-8') as f:f.write(proc.stdout)
 require(proc.returncode==0,name+': '+proc.stderr)
 a=FitnessAudit(c,50000);receipt=None;verified=False
 for line in proc.stdout.splitlines():
  r=json.loads(line)
  if r['kind']=='intervention_audit':receipt=r['receipt'];require(r['invalid_calls_rejected'],'invalid API accepted')
  elif r['kind']=='verification':require(r['shadow_equal'] is None and r['checkpoints']==0,'unexpected shadow mode');verified=True
  else:a.accept(r)
 s=a.finish();require(receipt is not None and verified,'incomplete verification')
 focal=tuple(receipt['id']);desc={focal}
 for child,parent in a.parents.items():
  if parent in desc:desc.add(child)
 s.update(value=value,ticks=50000,receipt=receipt,shadow_equal=None,
  focal_direct_births=len(a.life[focal]['children']),focal_descendant_births=len(desc)-1,
  focal_censored=a.life[focal]['end'] is None,
  focal_removal_tick=a.life[focal]['end'],positive_descendants_at_end=sum(i in a.survivors and a.survivors[i]['hp']>0 for i in desc-{focal}))
 save(out/(name+'.summary.json'),s);return s

def run(out,runtime,resume):
 require(not out.exists(),'new output required');require(bytes_under(ROOT)<25_000_000_000,'workspace cap')
 require(sha256(runtime/'sim_wasm_bg.wasm')==WASM,'unqualified WASM')
 require(json.loads((resume/'execution.json').read_text(encoding='utf-8'))['runtime_sha256']['sim_wasm_bg.wasm']==WASM,'resume runtime mismatch')
 out.mkdir(parents=True);sig=software();sig['files']['tools/focal_intervention_worker.mjs']=sha256(WORKER)
 save(out/'software.json',dict(signature=sig,provenance=provenance()));archive_source(out,sig)
 runtime_hashes={p.name:sha256(p) for p in runtime.iterdir() if p.is_file()}
 status=dict(status='running',started_at=now(),ticks=50000,runtime_sha256=runtime_hashes)
 status['resumed_from']=str(resume);status['resume_execution_sha256']=sha256(resume/'execution.json')
 status['resume_source_sha256']=sha256(resume/'source-snapshot.zip')
 save(out/'execution.json',status);start=time.monotonic();rows=[]
 try:
  with ThreadPoolExecutor(max_workers=4) as pool:
   futures=[pool.submit(replay,(seed,t,v),out,runtime,start,resume) for seed in SEEDS for t in (False,True) for v in (.2,.8)]
   for f in as_completed(futures):
    s=f.result();rows.append(s);print(f"{len(rows)}/32 seed{s['seed']} toxin{s['toxic']} r{s['value']} births={s['focal_direct_births']} descendants={s['focal_descendant_births']} elapsed={time.monotonic()-start:.1f}s",flush=True)
    require(bytes_under(out)<100_000_000,'100MB cap')
  rows.sort(key=lambda s:(s['seed'],s['toxic'],s['value']))
  save(out/'summaries.json',rows);report=assess(rows);save(out/'assessment.json',report)
  for key in ('seed_differences','contrasts'):
   with (out/(key+'.csv')).open('x',newline='',encoding='utf-8') as f:
    w=csv.DictWriter(f,fieldnames=list(report[key][0]));w.writeheader();w.writerows(report[key])
  require({p.name:sha256(p) for p in runtime.iterdir() if p.is_file()}==runtime_hashes,'runtime changed')
  require(all(sha256(ROOT/name)==h for name,h in sig['files'].items()),'source changed during run')
  require(time.monotonic()-start<1800 and bytes_under(out)<100_000_000,'resource cap')
  status.update(status='completed',cases=32,elapsed_seconds=time.monotonic()-start,output_sha256={p.name:sha256(p) for p in out.iterdir() if p.is_file() and p.name!='execution.json'})
  save(out/'execution.json',status)
 except Exception as e:
  status.update(status='failed',error=str(e));save(out/'execution.json',status);raise

def audit(out):
 audit_genealogies(out)
 rows=json.loads((out/'summaries.json').read_text(encoding='utf-8'))
 require(all(s['ticks']==50000 for s in rows),'wrong duration')
 require(assess(rows)==json.loads((out/'assessment.json').read_text(encoding='utf-8')),'assessment mismatch')
 print('Long-term seed contrasts audit PASS')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);p.add_argument('--runtime',type=Path);p.add_argument('--resume',type=Path);p.add_argument('--audit',action='store_true');a=p.parse_args()
 audit(a.out.resolve()) if a.audit else run(a.out.resolve(),a.runtime.resolve(),a.resume.resolve())
