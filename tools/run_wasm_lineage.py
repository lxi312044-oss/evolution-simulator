"""Direct genealogy observations from one fixed WASM, with exact old-Web trajectory controls."""
import argparse
from concurrent.futures import ThreadPoolExecutor,as_completed
from collections import defaultdict
import gzip
import json
from pathlib import Path
import subprocess
import threading
import time
from run_longterm_resistance import ROOT,SEEDS,Audit,analyze,config_for,case_name,interval,signature,resistance_bin
from run_confirmation import save,require,provenance,archive_source,bytes_under,now
from run_metadata import sha256
OLD=ROOT/'results/longterm-wasm-study-20261107'
NATIVE=ROOT/'results/longterm-resistance-20261107'
WORKER=ROOT/'tools/wasm_lineage_worker.mjs'

def complete_cohorts(life,parents,horizon):
 descendants=defaultdict(int);grandchildren=defaultdict(int)
 for child,a in life.items():
  if a['founder']:continue
  ancestor=parents[child];depth=1
  while ancestor is not None:
   elder=life[ancestor]
   if a['birth']>elder['birth']+5000:break
   descendants[ancestor]+=1
   if depth==2:grandchildren[ancestor]+=1
   ancestor=parents.get(ancestor);depth+=1
 groups=defaultdict(lambda:dict(individuals=0,offspring_first5000=0,descendants_first5000=0,grandchildren_first5000=0,removed_before5000=0,resistance_sum=0.))
 for ident,a in life.items():
  if a['founder'] or a['birth']+5000>horizon:continue
  # Closed 5000-Tick birth strata: 1..5000, 5001..10000, ..., 40001..45000.
  c=groups[((a['birth']-1)//5000,resistance_bin(a['r']))];c['individuals']+=1;c['resistance_sum']+=a['r'];c['offspring_first5000']+=sum(t<=a['birth']+5000 for t in a['children']);c['descendants_first5000']+=descendants[ident];c['grandchildren_first5000']+=grandchildren[ident];c['removed_before5000']+=int(a['end'] is not None and a['end']<=a['birth']+5000)
 for c in groups.values():require(c['descendants_first5000']>=c['offspring_first5000']+c['grandchildren_first5000'],'descendant accounting mismatch')
 return [dict(birth_window=k[0],resistance_bin=k[1],**v) for k,v in sorted(groups.items())]

class FitnessAudit(Audit):
 def __init__(self,config,ticks):super().__init__(config,ticks);self.parents={}
 def accept(self,row):
  super().accept(row)
  if row['kind']=='birth':self.parents[tuple(row['id'])]=tuple(row['parent'])
 def finish(self):
  summary=super().finish();summary['complete_first5000_cohorts']=complete_cohorts(self.life,self.parents,self.ticks);return summary

def sources(runtime):
 s=signature();s['files']['tools/wasm_lineage_worker.mjs']=sha256(WORKER)
 from run_resistance_pilot import digest_json
 s['manifest_sha256']=digest_json(s['files']);s['observed_wasm_sha256']=sha256(runtime/'sim_wasm_bg.wasm');s['observed_js_sha256']=sha256(runtime/'sim_wasm.js');return s

def replay(case,ticks,runtime,out,start,shadow):
 name=case_name(*case);c=config_for(*case);a=FitnessAudit(c,ticks);reference=json.loads((OLD/'runs'/f'{name}.json').read_text(encoding='utf-8'));publics=0;verified=False
 with (out/'runs'/f'{name}.stderr.log').open('x',encoding='utf-8') as errors,gzip.open(out/'runs'/f'{name}.jsonl.gz','wt',encoding='utf-8') as log:
  remaining=1800-(time.monotonic()-start);require(remaining>0,'30-minute cap')
  proc=subprocess.Popen(['node',str(WORKER),str(NATIVE/'inputs'/f'{name}.json'),str(runtime),str(ticks),'on' if shadow else 'off'],cwd=ROOT,stdout=subprocess.PIPE,stderr=errors,text=True,encoding='utf-8');timer=threading.Timer(min(600,remaining),proc.kill);timer.start()
  try:
   for line in proc.stdout:
    row=json.loads(line)
    if row['kind']=='public':require(row['sample']==reference['samples'][publics],f'old Web trajectory changed: {name} checkpoint {publics}');publics+=1
    elif row['kind']=='verification':require(row['shadow_equal'] and row['checkpoints']==ticks//1000+1,'shadow state mismatch');verified=True
    else:a.accept(row)
    log.write(json.dumps(row,separators=(',',':'))+'\n')
   require(proc.wait()==0,f'{name} exited {proc.returncode}');require(publics==ticks//1000+1 and (not shadow or verified),'missing public/verification data')
  finally:
   timer.cancel()
   if proc.poll() is None:proc.kill()
   proc.wait();proc.stdout.close()
 summary=a.finish();summary.update(public_checkpoints_equal=publics,shadow_equal=verified if shadow else None);save(out/'runs'/f'{name}.summary.json',summary);return summary

def cohort_analysis(summaries):
 contrasts=[];details=[];rates=[]
 for s in summaries:
  for group in (0,1,2):
   ticks=s['final']['adult_ticks'][group];births=s['final']['births_by_bin'][group]
   rates.append(dict(seed=s['seed'],size=s['size'],toxic=s['toxic'],resistance_bin=group,adult_ticks=ticks,births=births,births_per1000_adult_ticks=1000*births/ticks if ticks else None))
 for size in (20,40):
  for toxic in (False,True):
   for metric in ('offspring_first5000','descendants_first5000','grandchildren_first5000'):
    for comparison in (1,0):
     diffs=[];missing=[]
     for seed in SEEDS:
      s=next(x for x in summaries if (x['seed'],x['size'],x['toxic'])==(seed,size,toxic));groups={(c['birth_window'],c['resistance_bin']):c for c in s['complete_first5000_cohorts']};window_diffs=[]
      for window in range(9):
       high=groups.get((window,2));control=groups.get((window,comparison))
       if high and control:
        delta=high[metric]/high['individuals']-control[metric]/control['individuals'];window_diffs.append(delta);details.append(dict(seed=seed,size=size,toxic=toxic,birth_window=window,metric=metric,comparison_bin=comparison,high_n=high['individuals'],comparison_n=control['individuals'],high_per_individual=high[metric]/high['individuals'],comparison_per_individual=control[metric]/control['individuals'],difference=delta))
      if window_diffs:diffs.append(sum(window_diffs)/len(window_diffs))
      else:missing.append(seed)
     contrasts.append(dict(size=size,toxic=toxic,metric=metric,comparison_bin=comparison,missing_seeds=missing,**interval(diffs)))
 return dict(contrasts=contrasts,strata=details,reproductive_rates=rates,limitations=['No causal isolation of resistance from other genes/environment','High/low groups may be sparse; all available strata are retained without outcome-based threshold selection','Direct offspring and all descendants are separately counted in a complete 5000-Tick window; not lifetime fitness','Seed equal weighting over available birth strata; missing seed/strata disclosed'])

def run(out,runtime,ticks,shadow):
 require(not out.exists(),'use a new directory');require(bytes_under(ROOT)<25_000_000_000,'workspace cap');out.mkdir(parents=True);(out/'runs').mkdir();sig=sources(runtime);save(out/'software.json',dict(signature=sig,provenance=provenance()));archive_source(out,sig)
 cases=[(seed,size,toxic) for seed in (SEEDS if ticks==50000 else SEEDS[:1]) for size in (20,40) for toxic in (False,True)]
 status=dict(status='running',started_at=now(),ticks=ticks,cases=cases,runtime=str(runtime.relative_to(ROOT)),signature=sig,shadow=shadow,old_execution_sha256=sha256(OLD/'execution.json'));save(out/'execution.json',status);start=time.monotonic();summaries=[]
 try:
  with ThreadPoolExecutor(max_workers=4) as pool:
   for f in as_completed([pool.submit(replay,c,ticks,runtime,out,start,shadow or (ticks==50000 and c==(12345,40,True))) for c in cases]):
    s=f.result();summaries.append(s);print(f'WASM genealogy {len(summaries)}/{len(cases)} {case_name(s["seed"],s["size"],s["toxic"])}: births={s["births"]}, {time.monotonic()-start:.1f}s',flush=True);require(bytes_under(out)<200_000_000,'200MB cap')
  summaries.sort(key=lambda s:(s['seed'],s['size'],s['toxic']));save(out/'summaries.json',summaries)
  if ticks==50000:save(out/'assessment.json',analyze(summaries));save(out/'cohort_assessment.json',cohort_analysis(summaries))
  require(sources(runtime)==sig,'execution sources changed');status.update(status='completed',completed_at=now(),elapsed_seconds=time.monotonic()-start,runs=len(summaries),output_sha256={p.relative_to(out).as_posix():sha256(p) for p in out.rglob('*') if p.is_file() and p.name!='execution.json'});save(out/'execution.json',status)
 except Exception as error:status.update(status='failed',error=str(error));save(out/'execution.json',status);raise

def audit_saved(out):
 e=json.loads((out/'execution.json').read_text(encoding='utf-8'));require(e['status']=='completed','incomplete experiment');require(sha256(OLD/'execution.json')==e['old_execution_sha256'],'old study changed')
 for name,h in e['output_sha256'].items():require(sha256(out/name)==h,'saved output changed')
 summaries=[]
 for case in e['cases']:
  name=case_name(*case);a=FitnessAudit(config_for(*case),e['ticks']);old=json.loads((OLD/'runs'/f'{name}.json').read_text(encoding='utf-8'));count=0;verified=False
  with gzip.open(out/'runs'/f'{name}.jsonl.gz','rt',encoding='utf-8') as f:
   for line in f:
    row=json.loads(line)
    if row['kind']=='public':require(row['sample']==old['samples'][count],'saved old trajectory mismatch');count+=1
    elif row['kind']=='verification':require(row['shadow_equal'],'saved shadow mismatch');verified=True
    else:a.accept(row)
  s=a.finish();s.update(public_checkpoints_equal=count,shadow_equal=verified if (e['shadow'] or (e['ticks']==50000 and tuple(case)==(12345,40,True))) else None);require(s==json.loads((out/'runs'/f'{name}.summary.json').read_text(encoding='utf-8')),'saved individual summary mismatch');summaries.append(s)
 summaries.sort(key=lambda s:(s['seed'],s['size'],s['toxic']));require(summaries==json.loads((out/'summaries.json').read_text(encoding='utf-8')),'saved summaries mismatch')
 if e['ticks']==50000:require(analyze(summaries)==json.loads((out/'assessment.json').read_text(encoding='utf-8')),'saved assessment mismatch');require(cohort_analysis(summaries)==json.loads((out/'cohort_assessment.json').read_text(encoding='utf-8')),'saved cohort mismatch')
 save(out/'saved_log_audit.json',dict(passed=True,runs=len(summaries),execution_sha256=sha256(out/'execution.json')));print('WASM genealogy saved audit PASS')
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--runtime',type=Path,default=ROOT/'results/wasm-lineage-20261108/instrumented-runtime-final');p.add_argument('--ticks',type=int,choices=[1000,6000,50000],default=50000);p.add_argument('--shadow',action='store_true');p.add_argument('--audit',action='store_true');a=p.parse_args();audit_saved(a.out.resolve()) if a.audit else run(a.out.resolve(),a.runtime.resolve(),a.ticks,a.shadow)
