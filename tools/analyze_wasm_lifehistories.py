"""Decompose observed WASM life histories without changing or rerunning the simulation."""
import argparse
from collections import defaultdict
import csv
import gzip
import json
from pathlib import Path
import statistics
import time
from run_wasm_lineage import FitnessAudit
from run_longterm_resistance import ROOT,SEEDS,resistance_bin,interval,config_for,case_name
from run_confirmation import require,save,archive_source,provenance,bytes_under,software,now
from run_metadata import sha256
from run_resistance_pilot import digest_json
SOURCE=ROOT/'results/wasm-lineage-study-20261108'

class LifeHistoryAudit(FitnessAudit):
 def __init__(self,config,ticks):super().__init__(config,ticks);self.terminals={};self.founder_age={}
 def accept(self,row):
  super().accept(row)
  if row['kind']=='initial':self.founder_age={tuple(a['id']):a['age'] for a in row['founders']}
  if row['kind'] in ('removal','survivor'):
   ident=tuple(row['id']);life=self.life[ident];expected=row['tick']-life['birth']+self.founder_age.get(ident,0)
   require(row['age']==expected,'terminal age does not match birth time')
   require(ident not in self.terminals,'duplicate life terminal');self.terminals[ident]=row
 def records(self):
  require(set(self.terminals)==set(self.life),'missing terminal records');records=[]
  for ident,a in self.life.items():
   if a['founder']:continue
   t=self.terminals[ident];ages=[tick-a['birth'] for tick in a['children']];duration=t['tick']-a['birth'];censored=t['kind']=='survivor'
   require(all(0<=x<=duration for x in ages) and ages==sorted(ages),'offspring age outside observed life')
   records.append(dict(id=list(ident),birth_tick=a['birth'],resistance=a['r'],resistance_bin=resistance_bin(a['r']),duration=duration,censored=censored,
    observed_births=len(ages),lifetime_births=None if censored else len(ages),first_birth_age=ages[0] if ages else None,offspring_ages=ages,
    removal_trigger=None if censored else t['trigger'],terminal_hp=t['hp']))
  return records

def individual_window(a,horizon,min_age):
 require(horizon>0 and min_age>=0 and (not a['censored'] or a['duration']>=horizon),'window not fully observed')
 duration=min(a['duration'],horizon);births=sum(x<=horizon for x in a['offspring_ages']);first=a['first_birth_age']
 bred=first is not None and first<=horizon;removed=not a['censored'] and a['duration']<=horizon
 mature=max(0,duration-min_age)
 return dict(individuals=1,births=births,births_0_5000=sum(x<=min(horizon,5000) for x in a['offspring_ages']),births_after5000=sum(5000<x<=horizon for x in a['offspring_ages']),
  bred=int(bred),bred_by1000=int(first is not None and first<=min(horizon,1000)),bred_by2000=int(first is not None and first<=min(horizon,2000)),
  first_birth_sum=first if bred else 0,restricted_first_birth_wait=first if bred else horizon,
  restricted_duration=duration,mature_duration=mature,removed=int(removed),removed_before_first=int(removed and not bred),
  age_limit_removed=int(removed and a['removal_trigger']=='age_limit'),hp_removed=int(removed and a['removal_trigger']=='hp_nonpositive'),
  births_by_nonmature_time=int(births>0 and mature==0))

def summarize_window(rows,horizon,min_age):
 groups=defaultdict(lambda:defaultdict(float))
 for a in rows:
  if a['birth_tick']>50000-horizon:continue
  stats=individual_window(a,horizon,min_age);g=groups[((a['birth_tick']-1)//5000,a['resistance_bin'])]
  for key,value in stats.items():g[key]+=value
 result=[]
 for (window,b),s in sorted(groups.items()):
  n=s['individuals'];mean=lambda k:s[k]/n
  r=dict(horizon=horizon,birth_window=window,resistance_bin=b,individuals=int(n),births_per_individual=mean('births'),births_0_5000_per_individual=mean('births_0_5000'),births_after5000_per_individual=mean('births_after5000'),
   bred_fraction=mean('bred'),bred_by1000_fraction=mean('bred_by1000'),bred_by2000_fraction=mean('bred_by2000'),
   first_birth_mean_among_breeders=s['first_birth_sum']/s['bred'] if s['bred'] else None,restricted_first_birth_wait=mean('restricted_first_birth_wait'),
   restricted_duration=mean('restricted_duration'),mature_duration=mean('mature_duration'),removed_fraction=mean('removed'),removed_before_first_fraction=mean('removed_before_first'),
   age_limit_removed_fraction=mean('age_limit_removed'),hp_removed_fraction=mean('hp_removed'),births_per1000_mature_persistence=1000*s['births']/s['mature_duration'] if s['mature_duration'] else None,
   nonmature_birth_cases=int(s['births_by_nonmature_time']))
  require(abs(r['births_0_5000_per_individual']+r['births_after5000_per_individual']-r['births_per_individual'])<1e-10,'birth window decomposition mismatch')
  if s['mature_duration']:require(abs(r['mature_duration']*r['births_per1000_mature_persistence']/1000-r['births_per_individual'])<1e-10,'duration/rate product mismatch')
  require(abs(r['age_limit_removed_fraction']+r['hp_removed_fraction']-r['removed_fraction'])<1e-10,'removal trigger decomposition mismatch')
  result.append(r)
 return result

def lifetime_descriptive(rows):
 result=[]
 for b in (0,1,2):
  a=[x for x in rows if x['resistance_bin']==b];dead=[x for x in a if not x['censored']];live=[x for x in a if x['censored']]
  result.append(dict(resistance_bin=b,individuals=len(a),removed_complete_lives=len(dead),censored=len(live),
   confirmed_lifetime_birth_mean_among_removed=statistics.mean(x['lifetime_births'] for x in dead) if dead else None,
   observed_births_in_censored=sum(x['observed_births'] for x in live),unknown_future_births_in_censored=None if live else 0,
   observed_birth_lower_bound_per_individual=statistics.mean(x['observed_births'] for x in a) if a else None,
   duration_mean_among_removed=statistics.mean(x['duration'] for x in dead) if dead else None,
   first_birth_age_mean_among_observed_breeders=statistics.mean(x['first_birth_age'] for x in a if x['first_birth_age'] is not None) if any(x['first_birth_age'] is not None for x in a) else None))
 return result

def comparisons(cases):
 metrics=['births_per_individual','births_after5000_per_individual','bred_fraction','bred_by1000_fraction','bred_by2000_fraction','first_birth_mean_among_breeders','restricted_first_birth_wait','restricted_duration','mature_duration','removed_fraction','removed_before_first_fraction','age_limit_removed_fraction','hp_removed_fraction','births_per1000_mature_persistence']
 contrasts=[];strata=[]
 for horizon in (5000,10000):
  for size in (20,40):
   for toxic in (False,True):
    for metric in metrics:
     diffs=[];missing=[]
     for seed in SEEDS:
      case=next(c for c in cases if (c['seed'],c['size'],c['toxic'])==(seed,size,toxic));g={(x['birth_window'],x['resistance_bin']):x for x in case['windows'] if x['horizon']==horizon};d=[]
      for window in range((50000-horizon)//5000):
       h=g.get((window,2));m=g.get((window,1))
       if h and m and h[metric] is not None and m[metric] is not None:
        value=h[metric]-m[metric];d.append(value);strata.append(dict(horizon=horizon,seed=seed,size=size,toxic=toxic,birth_window=window,metric=metric,high=h[metric],middle=m[metric],high_n=h['individuals'],middle_n=m['individuals'],difference=value))
      if d:diffs.append(sum(d)/len(d))
      else:missing.append(seed)
     contrasts.append(dict(horizon=horizon,size=size,toxic=toxic,metric=metric,missing_seeds=missing,**interval(diffs)))
 return dict(contrasts=contrasts,strata=strata,limitations=['Removal time is not first HP0 time','Mature persistence is an age-based proxy, not exact positive-HP adult person-time','First-birth mean among breeders is conditional; never-breeders and removed-before-first are shown separately','Removed-only lifetime summaries are selected observations, not population lifetime fitness','Within-cohort association does not isolate resistance from other genes/family/environment'])

def source_signature():
 s=software();s['files']['tools/analyze_wasm_lifehistories.py']=sha256(ROOT/'tools/analyze_wasm_lifehistories.py');s['manifest_sha256']=digest_json(s['files']);return s

def run(out):
 require(not out.exists(),'use a new output directory');require(bytes_under(ROOT)<25_000_000_000,'workspace cap')
 execution=json.loads((SOURCE/'execution.json').read_text(encoding='utf-8'));require(execution['status']=='completed' and execution['runs']==32,'source incomplete')
 for name,h in execution['output_sha256'].items():require(sha256(SOURCE/name)==h,'source output changed')
 out.mkdir(parents=True);sig=source_signature();save(out/'software.json',dict(signature=sig,provenance=provenance()));archive_source(out,sig)
 status=dict(status='running',started_at=now(),source_execution_sha256=sha256(SOURCE/'execution.json'),source_audit_sha256=sha256(SOURCE/'saved_log_audit.json'),signature=sig);save(out/'execution.json',status);start=time.monotonic();cases=[]
 try:
  with gzip.open(out/'individual_lives.jsonl.gz','wt',encoding='utf-8') as individuals:
   for case in execution['cases']:
    require(time.monotonic()-start<900,'15-minute cap');name=case_name(*case);a=LifeHistoryAudit(config_for(*case),50000);publics=0;verified=False
    with gzip.open(SOURCE/'runs'/f'{name}.jsonl.gz','rt',encoding='utf-8') as f:
     for line in f:
      row=json.loads(line)
      if row['kind']=='public':publics+=1
      elif row['kind']=='verification':verified=row['shadow_equal']
      else:a.accept(row)
    old=a.finish();old.update(public_checkpoints_equal=publics,shadow_equal=verified if (execution['shadow'] or tuple(case)==(12345,40,True)) else None)
    require(old==json.loads((SOURCE/'runs'/f'{name}.summary.json').read_text(encoding='utf-8')),'previous genealogy summary not reproduced')
    records=a.records();min_age=a.initial['config']['herbivores']['reproduction_min_age'];windows=summarize_window(records,5000,min_age)+summarize_window(records,10000,min_age)
    # Independent reconstruction of the primary 5000-Tick direct-birth cohort totals.
    prior={(c['birth_window'],c['resistance_bin']):c for c in old['complete_first5000_cohorts']}
    for w in windows:
     if w['horizon']==5000:
      c=prior[(w['birth_window'],w['resistance_bin'])];require(c['individuals']==w['individuals'] and abs(c['offspring_first5000']/c['individuals']-w['births_per_individual'])<1e-10,'prior cohort mismatch')
    summary=dict(seed=case[0],size=case[1],toxic=case[2],min_age=min_age,individuals=len(records),founders_excluded=sum(x['founder'] for x in a.life.values()),windows=windows,lifetime=lifetime_descriptive(records))
    save(out/f'{name}.json',summary);cases.append(summary)
    for record in records:individuals.write(json.dumps(dict(seed=case[0],size=case[1],toxic=case[2],**record),separators=(',',':'))+'\n')
    print(f'Validated {name}: {len(records)} complete/censored life histories',flush=True)
  report=comparisons(cases);save(out/'assessment.json',report)
  original=json.loads((SOURCE/'cohort_assessment.json').read_text(encoding='utf-8'))
  for c in original['contrasts']:
   if c['metric']=='offspring_first5000' and c['comparison_bin']==1:
    actual=next(x for x in report['contrasts'] if (x['horizon'],x['size'],x['toxic'],x['metric'])==(5000,c['size'],c['toxic'],'births_per_individual'))
    require(actual['n']==c['n'] and actual['missing_seeds']==c['missing_seeds'] and abs(actual['mean']-c['mean'])<1e-10,'original primary contrast mismatch')
  for filename,rows in [('contrasts.csv',report['contrasts']),('strata.csv',report['strata']),('window_groups.csv',[dict(seed=c['seed'],size=c['size'],toxic=c['toxic'],**w) for c in cases for w in c['windows']]),('lifetime_observed.csv',[dict(seed=c['seed'],size=c['size'],toxic=c['toxic'],**x) for c in cases for x in c['lifetime']])]:
   with (out/filename).open('x',newline='',encoding='utf-8') as f:w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
  require(source_signature()==sig,'analysis sources changed');require(bytes_under(out)<100_000_000,'100MB cap')
  status.update(status='completed',completed_at=now(),elapsed_seconds=time.monotonic()-start,cases=len(cases),individuals=sum(c['individuals'] for c in cases),output_sha256={p.name:sha256(p) for p in out.iterdir() if p.is_file() and p.name!='execution.json'});save(out/'execution.json',status)
 except Exception as error:status.update(status='failed',error=str(error));save(out/'execution.json',status);raise

def audit(out):
 e=json.loads((out/'execution.json').read_text(encoding='utf-8'));require(e['status']=='completed' and e['cases']==32,'analysis incomplete')
 for name,h in e['output_sha256'].items():require(sha256(out/name)==h,'saved analysis changed')
 require(sha256(SOURCE/'execution.json')==e['source_execution_sha256'] and sha256(SOURCE/'saved_log_audit.json')==e['source_audit_sha256'],'source metadata changed')
 cases=[json.loads((out/f'{case_name(*case)}.json').read_text(encoding='utf-8')) for case in json.loads((SOURCE/'execution.json').read_text(encoding='utf-8'))['cases']]
 require(comparisons(cases)==json.loads((out/'assessment.json').read_text(encoding='utf-8')),'saved contrast recomputation mismatch')
 # Verify the individual-record artifact independently regenerates each group's windows and lifetime summaries.
 grouped=defaultdict(list)
 with gzip.open(out/'individual_lives.jsonl.gz','rt',encoding='utf-8') as f:
  for line in f:r=json.loads(line);grouped[(r['seed'],r['size'],r['toxic'])].append(r)
 for c in cases:
  r=grouped[(c['seed'],c['size'],c['toxic'])];require(summarize_window(r,5000,c['min_age'])+summarize_window(r,10000,c['min_age'])==c['windows'],'saved windows mismatch');require(lifetime_descriptive(r)==c['lifetime'],'saved lifetime summary mismatch')
 save(out/'saved_log_audit.json',dict(passed=True,cases=32,individuals=e['individuals'],execution_sha256=sha256(out/'execution.json')));print('Saved life-history audit PASS')
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--audit',action='store_true');a=p.parse_args();audit(a.out.resolve()) if a.audit else run(a.out.resolve())
