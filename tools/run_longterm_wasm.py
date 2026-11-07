"""Frozen supplementary Web WASM batch; invocation: python <this file> [--audit]."""
import sys,json,math,time,subprocess,argparse,shutil
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor,as_completed
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'tools'))
from run_longterm_resistance import config_for,case_name,SEEDS,interval
from run_confirmation import require,save,now,bytes_under
from run_metadata import sha256
OUT=ROOT/'results/longterm-wasm-study-20261107';NATIVE=ROOT/'results/longterm-resistance-20261107';RUNTIME=ROOT/'web/src/pkg';WORKER=ROOT/'tools/longterm_wasm_worker.mjs'
CASES=[(s,size,toxic) for s in SEEDS for size in (20,40) for toxic in (False,True)]
def validate(row,case):
 require(row['ticks']==50000,'horizon mismatch');require(row['wasm_sha256']==sha256(RUNTIME/'sim_wasm_bg.wasm') and row['js_sha256']==sha256(RUNTIME/'sim_wasm.js'),'runtime mismatch');require(row['config']==config_for(*case),'config mismatch');require([s['tick'] for s in row['samples']]==list(range(0,50001,1000)),'missing checkpoints')
 for s in row['samples']:
  require(all(v is None or not isinstance(v,float) or math.isfinite(v) for v in s.values()),'nonfinite data');require(0<=s['positive']<=s['present'] and 0<=s['high']<=s['present'],'invalid count')
  require(abs(s['n_error'])<=max(.001,1e-9*abs(row['initial_n'])) and abs(s['e_error'])<=max(.001,1e-9*(abs(row['initial_e'])+abs(s['sunlight_in'])+abs(s['heat_out']))),'exported resource audit failure')
  require(math.isclose(s['total_n']-row['initial_n'],s['n_error'],abs_tol=1e-9) and math.isclose(s['total_e']-row['initial_e']-s['sunlight_in']+s['heat_out'],s['e_error'],abs_tol=1e-9),'resource residual mismatch')
  if s['present']:require(0<=s['resistance_mean']<=1 and s['high_fraction']==s['high']/s['present'],'invalid resistance')
  else:require(s['resistance_mean'] is None and s['high_fraction'] is None,'extinction must be missing')
  if not case[2]:require(s['dose']==0 and s['damage']==0,'toxin OFF mismatch')
 require(row['samples'][0]['present']==row['config']['initial_population']['herbivores'] and row['samples'][0]['resistance_mean']==.5,'initial mismatch')
 require(row['input_sha256']==sha256(NATIVE/'inputs'/f'{case_name(*case)}.json'),'input changed')
 return row

def analyze(rows):
 index={(r['config']['seed'],r['config']['world']['land_columns'],r['config']['toxin']['enabled']):r for r in rows};groups=[];contrasts=[]
 for size in (20,40):
  for toxic in (False,True):
   last=[index[(s,size,toxic)]['samples'][-1] for s in SEEDS]
   groups.append(dict(size=size,toxic=toxic,runs=8,extinct=sum(r['present']==0 for r in last),mean_resistance=interval([r['resistance_mean'] for r in last if r['resistance_mean'] is not None]),high_fraction=interval([r['high_fraction'] for r in last if r['high_fraction'] is not None]),mean_population=sum(r['present'] for r in last)/8,predators_extinct=sum(r['carnivores']==0 for r in last),mean_births=sum(r['births'] for r in last)/8,mean_dose=sum(r['dose'] for r in last)/8))
  for tick in (6000,10000,20000,30000,40000,50000):
   for key in ('resistance_mean','high_fraction'):
    diffs=[];missing=[]
    for seed in SEEDS:
     pair=[index[(seed,size,toxic)]['samples'][tick//1000][key] for toxic in (False,True)]
     if None in pair:missing.append(seed)
     else:diffs.append(pair[1]-pair[0])
    contrasts.append(dict(size=size,tick=tick,metric=key,missing_seeds=missing,**interval(diffs)))
 return dict(runtime='existing Web WASM, executed in Node',groups=groups,paired_contrasts=contrasts,independent_seeds=8,runs=32,max_n_error=max(abs(s['n_error']) for r in rows for s in r['samples']),max_e_error=max(abs(s['e_error']) for r in rows for s in r['samples']),generation_data_available=False,energy_audit_note='Web API exports raw ledger sums; residual is not the compensated internal ledger error; existing max(abs=1e-3,rel=1e-9) scale is retained')

def run():
 require(not (OUT/'execution.json').exists(),'refuse overwrite');require(json.loads((NATIVE/'execution.json').read_text())['status']=='completed','native batch must finish first')
 OUT.mkdir(parents=True,exist_ok=False);(OUT/'runs').mkdir();(OUT/'runtime').mkdir()
 for name in ('sim_wasm.js','sim_wasm_bg.wasm','package.json'):shutil.copyfile(RUNTIME/name,OUT/'runtime'/name)
 shutil.copyfile(WORKER,OUT/'worker-source.mjs');shutil.copyfile(Path(__file__),OUT/'runner-source.py')
 start=time.monotonic();sig={str(p.relative_to(ROOT)):sha256(p) for p in [WORKER,Path(__file__),RUNTIME/'sim_wasm.js',RUNTIME/'sim_wasm_bg.wasm',ROOT/'tools/run_longterm_resistance.py',NATIVE/'execution.json']}
 status=dict(status='running',started_at=now(),input_sha256=sig,cases=CASES);save(OUT/'execution.json',status)
 def execute(case):
  name=case_name(*case);remaining=1800-(time.monotonic()-start);require(remaining>0,'30-minute batch cap')
  p=subprocess.run(['node',str(WORKER),str(NATIVE/'inputs'/f'{name}.json'),str(OUT/'runs'/f'{name}.json'),'50000',str(RUNTIME)],cwd=ROOT,capture_output=True,text=True,encoding='utf-8',timeout=min(610,remaining));(OUT/'runs'/f'{name}.stderr.log').write_text(p.stderr,encoding='utf-8');require(p.returncode==0,f'{name}: {p.stderr[-600:]}')
  return validate(json.loads((OUT/'runs'/f'{name}.json').read_text(encoding='utf-8')),case)
 try:
  rows=[]
  with ThreadPoolExecutor(max_workers=4) as pool:
   for f in as_completed([pool.submit(execute,c) for c in CASES]):
    r=f.result();rows.append(r);print(f'WASM validated {len(rows)}/32: seed{r["config"]["seed"]} size{r["config"]["world"]["land_columns"]} toxin{r["config"]["toxin"]["enabled"]} mean={r["samples"][-1]["resistance_mean"]}; {time.monotonic()-start:.1f}s',flush=True);require(bytes_under(OUT)<200_000_000,'200MB cap')
  rows.sort(key=lambda r:(r['config']['seed'],r['config']['world']['land_columns'],r['config']['toxin']['enabled']));index={(r['config']['seed'],r['config']['world']['land_columns'],r['config']['toxin']['enabled']):r for r in rows}
  for seed in SEEDS:
   for size in (20,40):require(index[(seed,size,False)]['initial_public_sha256']==index[(seed,size,True)]['initial_public_sha256'],'paired public initial state mismatch')
  for name,digest in sig.items():require(sha256(ROOT/name)==digest,'execution inputs changed')
  save(OUT/'assessment.json',analyze(rows));status.update(status='completed',completed_at=now(),elapsed_seconds=time.monotonic()-start,runs=32,output_sha256={p.relative_to(OUT).as_posix():sha256(p) for p in OUT.rglob('*') if p.is_file() and p.name!='execution.json'});save(OUT/'execution.json',status)
 except Exception as error:status.update(status='failed',error=str(error));save(OUT/'execution.json',status);raise

def audit():
 e=json.loads((OUT/'execution.json').read_text());require(e['status']=='completed' and e['runs']==32,'incomplete batch')
 for name,digest in e['input_sha256'].items():require(sha256(ROOT/name)==digest,'input changed')
 for name,digest in e['output_sha256'].items():require(sha256(OUT/name)==digest,'output changed')
 rows=[validate(json.loads((OUT/'runs'/f'{case_name(*c)}.json').read_text(encoding='utf-8')),c) for c in CASES];require(analyze(rows)==json.loads((OUT/'assessment.json').read_text()),'saved analysis mismatch');save(OUT/'saved_log_audit.json',dict(passed=True,runs=32,execution_sha256=sha256(OUT/'execution.json')));print('WASM saved audit PASS')
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--native',type=Path,default=NATIVE);p.add_argument('--runtime',type=Path,default=RUNTIME);p.add_argument('--audit',action='store_true');a=p.parse_args();OUT=a.out.resolve();NATIVE=a.native.resolve();RUNTIME=a.runtime.resolve();audit() if a.audit else run()
