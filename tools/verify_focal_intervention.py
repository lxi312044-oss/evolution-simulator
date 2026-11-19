"""Issue 35 short WASM intervention qualification; no scientific fitness verdict."""
import argparse,gzip,json,subprocess,time
from pathlib import Path
from run_longterm_resistance import ROOT,config_for,SEEDS
from run_wasm_lineage import FitnessAudit
from run_confirmation import save,require,now,software,archive_source,provenance
from run_metadata import sha256

def run(out,runtime):
 require(not out.exists(),'new output required')
 out.mkdir(parents=True);start=time.monotonic();sig=software()
 for name in ['focal_intervention_worker.mjs','verify_focal_intervention.py']:
  sig['files']['tools/'+name]=sha256(ROOT/'tools'/name)
 save(out/'software.json',dict(signature=sig,provenance=provenance()))
 archive_source(out,sig)
 status=dict(status='running',started_at=now(),runtime_sha256={p.name:sha256(p) for p in runtime.iterdir() if p.is_file()})
 save(out/'execution.json',status);summaries=[];initials={}
 try:
  for seed in SEEDS:
   for toxic in (False,True):
    for value in (.2,.8):
     ticks=6000 if seed==SEEDS[0] else 1000
     name=f'seed{seed}_toxin{int(toxic)}_r{value}'
     c=config_for(seed,40,toxic);save(out/(name+'.input.json'),c)
     a=FitnessAudit(c,ticks);receipt=None;verified=False
     proc=subprocess.run(['node',str(ROOT/'tools/focal_intervention_worker.mjs'),str(out/(name+'.input.json')),str(runtime),str(value),str(ticks)],capture_output=True,text=True,encoding='utf-8',timeout=120)
     (out/(name+'.stderr.log')).write_text(proc.stderr,encoding='utf-8')
     require(proc.returncode==0,name+': '+proc.stderr)
     with gzip.open(out/(name+'.jsonl.gz'),'wt',encoding='utf-8') as f:
      f.write(proc.stdout)
     for line in proc.stdout.splitlines():
      r=json.loads(line)
      if r['kind']=='intervention_audit':
       receipt=r['receipt'];key=(seed,toxic)
       if key in initials:require(initials[key]==r['before'],'pair initial state mismatch')
       else:initials[key]=r['before']
      elif r['kind']=='verification':require(r['shadow_equal'] and r['checkpoints']==ticks//1000+1,'shadow verification');verified=True
      else:a.accept(r)
     summary=a.finish();require(receipt is not None and verified,'incomplete audit')
     focal=tuple(receipt['id']);desc={focal}
     for child,parent in a.parents.items():
      if parent in desc:desc.add(child)
     summary.update(value=value,ticks=ticks,receipt=receipt,shadow_equal=verified,
       focal_direct_births=len(a.life[focal]['children']),focal_descendant_births=len(desc)-1,
       focal_censored=a.life[focal]['end'] is None)
     save(out/(name+'.summary.json'),summary);summaries.append(summary)
     require(time.monotonic()-start<1800,'30-minute cap')
     require(sum(p.stat().st_size for p in out.iterdir())<100_000_000,'100MB cap')
     print(name+' PASS',flush=True)
  require(len(summaries)==32,'case count')
  save(out/'summaries.json',summaries)
  status.update(status='completed',cases=32,elapsed_seconds=time.monotonic()-start,
    output_sha256={p.name:sha256(p) for p in out.iterdir() if p.is_file() and p.name!='execution.json'})
  save(out/'execution.json',status)
 except Exception as e:
  status.update(status='failed',error=str(e));save(out/'execution.json',status);raise

def audit(out):
 e=json.loads((out/'execution.json').read_text(encoding='utf-8'));require(e['status']=='completed','incomplete')
 for name,h in e['output_sha256'].items():require(sha256(out/name)==h,'artifact hash changed')
 count=0;initials={};keys=set()
 for p in out.glob('*.input.json'):
  name=p.name.removesuffix('.input.json');s=json.loads((out/(name+'.summary.json')).read_text(encoding='utf-8'))
  a=FitnessAudit(json.loads(p.read_text(encoding='utf-8')),s['ticks']);verified=False
  with gzip.open(out/(name+'.jsonl.gz'),'rt',encoding='utf-8') as f:
   for line in f:
    r=json.loads(line)
    if r['kind']=='intervention_audit':
     require(r['receipt']==s['receipt'] and r['invalid_calls_rejected'],'receipt changed')
     seed=s['seed'];initial=r['before']
     if seed in initials:require(initials[seed]==initial,'initial ecology differs across toxin/resistance arms')
     else:initials[seed]=initial
    elif r['kind']=='verification':
     expected_shadow=s['shadow_equal']
     require(r['shadow_equal'] is expected_shadow and r['checkpoints']==(s['ticks']//1000+1 if expected_shadow else 0),'saved shadow protocol mismatch')
     verified=True
    else:a.accept(r)
  actual=a.finish()
  require(all(actual[k]==s[k] for k in actual),'saved genealogy mismatch')
  focal=tuple(s['receipt']['id']);desc={focal}
  for child,parent in a.parents.items():
   if parent in desc:desc.add(child)
  require(len(a.life[focal]['children'])==s['focal_direct_births'] and len(desc)-1==s['focal_descendant_births'],'focal birth metrics mismatch')
  require((a.life[focal]['end'] is None)==s['focal_censored'],'focal censor mismatch')
  keys.add((s['seed'],s['toxic'],s['value']))
  require(verified,'missing shadow');count+=1
 require(count==32,'missing cases')
 require(keys=={(seed,toxic,value) for seed in SEEDS for toxic in (False,True) for value in (.2,.8)},'case matrix mismatch')
 save(out/'saved_log_audit.json',dict(passed=True,cases=count,execution_sha256=sha256(out/'execution.json'),audit_source_sha256=sha256(Path(__file__))))
 print('Saved qualification audit PASS')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);p.add_argument('--runtime',type=Path);p.add_argument('--audit',action='store_true');args=p.parse_args()
 audit(args.out.resolve()) if args.audit else run(args.out.resolve(),args.runtime.resolve())
