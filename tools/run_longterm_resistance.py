"""Paired Web-density long-run resistance study; population means are not proof of adaptation."""
import argparse
from concurrent.futures import ThreadPoolExecutor,as_completed
from collections import Counter,defaultdict
import csv
import gzip
import json
import math
from pathlib import Path
import random
import struct
import subprocess
import threading
import time
from run_confirmation import ROOT,software,provenance,archive_source,save,now,require,bytes_under
from run_resistance_pilot import digest_json
from run_metadata import sha256
BASE=ROOT/'config/standard-toxin.json'
BINARY=ROOT/'target/release/examples/resistance_longterm.exe'
SEEDS=list(range(12345,12353))
GERMINATION=dict(soil_threshold=20.,probability_per_cell=.005,check_interval=20,max_births_per_check=20)

def f32(x):return struct.unpack('f',struct.pack('f',x))[0]
def resistance_bin(r):return 2 if f32(r)>=f32(.6) else 0 if f32(r)<=f32(.4) else 1

def signature():
    s=software();s['files']['config/standard-toxin.json']=sha256(BASE);s['files']['web/src/field.ts']=sha256(ROOT/'web/src/field.ts')
    s['manifest_sha256']=digest_json(s['files']);s['experiment_binary_sha256']=sha256(BINARY);return s

def config_for(seed,size,toxic):
    c=json.loads(BASE.read_text(encoding='utf-8'));c['seed']=seed;c['toxin']['enabled']=toxic
    ratio=(size/20)**2;c['world'].update(width=1000*size/20,height=1000*size/20,land_columns=size,land_rows=size)
    for k in c['initial_population']:c['initial_population'][k]=round(c['initial_population'][k]*ratio)
    return c

def require_config(actual,expected):
    for key,value in expected.items():
        require(key in actual,f'missing config {key}')
        if isinstance(value,dict):require_config(actual[key],value)
        else:require(actual[key]==value or (isinstance(value,float) and isinstance(actual[key],(int,float)) and math.isclose(actual[key],value,rel_tol=1e-7,abs_tol=1e-9)),f'config mismatch {key}')

def finite(x):
    if isinstance(x,dict):return all(finite(v) for v in x.values())
    if isinstance(x,list):return all(finite(v) for v in x)
    return not isinstance(x,float) or math.isfinite(x)

class Audit:
    def __init__(self,config,ticks):
        self.config=config;self.ticks=ticks;self.life={};self.live=set();self.survivors={};self.samples=[];self.initial=None;self.complete=None
        self.parent_sum=0.;self.child_sum=0.;self.removed_totals=Counter();self.births=0;self.removals=0;self.mutations=0;self.by_bin=[0,0,0];self.last_tick=0;self.generation=0
    def accept(self,row):
        require(finite(row),'nonfinite data');kind=row['kind']
        if kind=='initial':
            require(self.initial is None and not self.life,'duplicate initial');require_config(row['config'],self.config)
            require(row['ticks']==self.ticks and row['interval']==1000 and row['germination']==GERMINATION,'protocol mismatch')
            self.initial=row
            for a in row['founders']:
                ident=tuple(a['id']);require(ident not in self.life,'duplicate founder');self.life[ident]=dict(r=a['resistance'],birth=0,generation=0,founder=True,children=[],end=None);self.live.add(ident)
            require(len(self.life)==self.config['initial_population']['herbivores'],'founder count mismatch');return
        require(self.initial is not None and self.complete is None,'missing initial or data after completion')
        tick=row['tick'];require(0<=tick<=self.ticks and tick>=self.last_tick,'event time reversed');self.last_tick=tick
        if kind=='birth':
            ident=tuple(row['id']);parent=tuple(row['parent']);require(ident not in self.life and parent in self.live,'invalid parent/child')
            p=self.life[parent];require(row['parent_resistance']==p['r'] and row['generation']==p['generation']+1,'inheritance identity mismatch')
            require(0<=row['resistance']<=1,'out of range resistance')
            if not row['mutation_attempted']:require(row['resistance']==p['r'],'unlogged mutation')
            p['children'].append(tick);self.life[ident]=dict(r=row['resistance'],birth=tick,generation=row['generation'],founder=False,children=[],end=None);self.live.add(ident)
            self.parent_sum+=p['r'];self.child_sum+=row['resistance'];self.births+=1;self.mutations+=int(row['mutation_attempted']);self.by_bin[resistance_bin(p['r'])]+=1;self.generation=max(self.generation,row['generation']);return
        if kind in ('removal','survivor'):
            ident=tuple(row['id']);require(ident in self.live,'duplicate/missing terminal');a=self.life[ident]
            require(row['resistance']==a['r'] and row['generation']==a['generation'] and row['births']==len(a['children']),'terminal genealogy mismatch')
            require(min(row['dose'],row['damage'],row['upkeep'])>=0,'negative lifetime total')
            if kind=='removal':
                self.live.remove(ident);self.removals+=1;a['end']=tick
                for key in ('dose','damage','upkeep'):self.removed_totals[key]+=row[key]
            else:require(tick==self.ticks and ident not in self.survivors,'invalid survivor');self.survivors[ident]=row
            return
        if kind=='snapshot':
            expected=0 if not self.samples else min(self.samples[-1]['tick']+1000,self.ticks)
            require(tick==expected and (not self.samples or tick>self.samples[-1]['tick']),'missing/duplicate checkpoint');n=len(self.live)
            require(row['present']==n and row['births']==self.births and row['removals']==self.removals and row['mutation_attempts']==self.mutations and row['births_by_bin']==self.by_bin and row['max_generation']==self.generation,'snapshot accounting mismatch')
            require(0<=row['positive']<=n and row['max_n_error']<=.001 and row['max_e_error']<=.001,'population/conservation failure')
            if n:
                mean=sum(self.life[i]['r'] for i in self.live)/n;high=sum(resistance_bin(self.life[i]['r'])==2 for i in self.live)
                require(row['resistance_mean'] is not None and abs(row['resistance_mean']-mean)<=1e-10 and row['high']==high and abs(row['high_fraction']-high/n)<=1e-10,'resistance summary mismatch')
            else:require(row['resistance_mean'] is None and row['high_fraction'] is None and row['variance'] is None,'extinction must be missing')
            if row['positive']==0:require(row['positive_mean'] is None and row['positive_high_fraction'] is None,'positive extinction must be missing')
            if self.samples:
                prior=self.samples[-1]
                for key in ['dose','damage','upkeep','germination_births','protected_ticks','starvation_ticks','capacity_ticks','adult_resistance_sum','parent_resistance_sum','child_resistance_sum']:require(row[key]+1e-7>=prior[key],f'counter reversed {key}')
                require(all(a>=b for a,b in zip(row['adult_ticks'],prior['adult_ticks'])),'adult exposure reversed')
            require(abs(row['parent_resistance_sum']-self.parent_sum)<=1e-7 and abs(row['child_resistance_sum']-self.child_sum)<=1e-7,'parent/child weighted sums mismatch')
            if not self.config['toxin']['enabled']:require(row['dose']==0 and row['damage']==0,'exposure/damage despite toxin OFF')
            self.samples.append(row);return
        if kind=='complete':
            require(tick==self.ticks and self.samples[-1]['tick']==self.ticks and self.live==set(self.survivors),'missing final data')
            require(row['births']==self.births and row['removals']==self.removals,'completion mismatch')
            self.complete=row;return
        raise ValueError(f'unknown record {kind}')
    def finish(self):
        require(self.complete is not None,'incomplete run');last=self.samples[-1]
        require(sum(a['hp']>0 for a in self.survivors.values())==last['positive'],'final positive count mismatch')
        totals={k:sum(a[k] for a in self.survivors.values()) for k in ('dose','damage','upkeep')}
        for key in totals:require(math.isclose(totals[key]+self.removed_totals[key],last[key],rel_tol=1e-10,abs_tol=1e-6),'lifetime exposure/cost sum mismatch')
        positive=[a for a in self.survivors.values() if a['hp']>0]
        if positive:require(abs(sum(a['resistance'] for a in positive)/len(positive)-last['positive_mean'])<1e-10,'final positive trait mean mismatch')
        # Mature birth cohorts: every offspring born by T-5000 has a complete first 5000-Tick birth window, even if it died earlier.
        cohorts=defaultdict(lambda:dict(individuals=0,offspring_first5000=0,removed_before5000=0,resistance_sum=0.))
        for a in self.life.values():
            if a['founder'] or a['birth']+5000>self.ticks:continue
            key=(a['birth']//5000,resistance_bin(a['r']));c=cohorts[key];c['individuals']+=1;c['resistance_sum']+=a['r'];c['offspring_first5000']+=sum(t<=a['birth']+5000 for t in a['children']);c['removed_before5000']+=int(a['end'] is not None and a['end']<=a['birth']+5000)
        return dict(seed=self.config['seed'],size=self.config['world']['land_columns'],toxic=self.config['toxin']['enabled'],
            initial_state_sha256=digest_json(self.initial['initial_state']),snapshots=self.samples,final=last,
            individuals=len(self.life),births=self.births,removals=self.removals,mutation_attempts=self.mutations,
            complete_first5000_cohorts=[dict(birth_window=k[0],resistance_bin=k[1],**v) for k,v in sorted(cohorts.items())],
            excluded_founders=sum(a['founder'] for a in self.life.values()),right_censored_offspring=sum(not a['founder'] and a['birth']+5000>self.ticks for a in self.life.values()))

def interval(values):
    if not values:return dict(n=0,mean=None,ci95=None)
    mean=sum(values)/len(values)
    if len(values)<2:return dict(n=len(values),mean=mean,ci95=None)
    rng=random.Random(20261107);b=sorted(sum(rng.choice(values) for _ in values)/len(values) for _ in range(4000))
    return dict(n=len(values),mean=mean,ci95=[b[99],b[3899]])

def analyze(summaries):
    indexed={(s['seed'],s['size'],s['toxic']):s for s in summaries};groups=[];contrasts=[]
    for size in (20,40):
        for toxic in (False,True):
            sub=[indexed[(seed,size,toxic)] for seed in SEEDS];final=[s['final'] for s in sub]
            groups.append(dict(size=size,toxic=toxic,runs=len(sub),extinct=sum(s['present']==0 for s in final),positive_extinct=sum(s['positive']==0 for s in final),
                final_mean=interval([s['resistance_mean'] for s in final if s['resistance_mean'] is not None]),final_high_fraction=interval([s['high_fraction'] for s in final if s['high_fraction'] is not None]),
                mean_herbivores=sum(s['present'] for s in final)/len(sub),predators_extinct=sum(s['carnivores']==0 for s in final),
                mean_max_generation=sum(s['max_generation'] for s in final)/len(sub),mean_births=sum(s['births'] for s in sub)/len(sub),mean_dose=sum(s['dose'] for s in final)/len(sub),
                parent_selection_association=interval([s['parent_resistance_sum']/s['births']-s['adult_resistance_sum']/sum(s['adult_ticks']) for s in final if s['births'] and sum(s['adult_ticks'])]),
                mutation_transmission_shift=interval([(s['child_resistance_sum']-s['parent_resistance_sum'])/s['births'] for s in final if s['births']])))
        for horizon in (6000,10000,20000,30000,40000,50000):
            for key in ('resistance_mean','high_fraction','positive_mean','positive_high_fraction'):
                diffs=[];missing=[]
                for seed in SEEDS:
                    pair=[next(s for s in indexed[(seed,size,t)]['snapshots'] if s['tick']==horizon)[key] for t in (False,True)]
                    if None in pair:missing.append(seed)
                    else:diffs.append(pair[1]-pair[0])
                contrasts.append(dict(size=size,tick=horizon,metric=key,estimand='toxin_ON_minus_OFF',missing_seeds=missing,**interval(diffs)))
    return dict(groups=groups,paired_contrasts=contrasts,independent_seeds=len(SEEDS),runs=len(summaries),limitations=['8 preselected seeds; exploratory, no automatic adaptation verdict','Size changes population size and relative pressure of fixed protection/capacity/germination caps','Toxin toggle changes synthesis cost and ecology as well as damage','Adult time-weighted reproductive association is not a controlled trait-specific causal effect','Founders excluded from complete birth-cohort analysis; late offspring right censored'])

def case_name(seed,size,toxic):return f'seed{seed}_size{size}_toxin{int(toxic)}'

def replay(case,ticks,out,started):
    seed,size,toxic=case;name=case_name(*case);c=config_for(*case);config_path=out/'inputs'/f'{name}.json';save(config_path,c)
    remaining=1800-(time.monotonic()-started);require(remaining>0,'30-minute batch cap');audit=Audit(c,ticks)
    with (out/'runs'/f'{name}.stderr.log').open('x',encoding='utf-8') as errors,gzip.open(out/'runs'/f'{name}.jsonl.gz','wt',encoding='utf-8') as log:
        proc=subprocess.Popen([str(BINARY),str(config_path),str(ticks)],cwd=ROOT,stdout=subprocess.PIPE,stderr=errors,text=True,encoding='utf-8');timer=threading.Timer(min(600,remaining),proc.kill);timer.start()
        try:
            for line in proc.stdout:
                row=json.loads(line);audit.accept(row);log.write(json.dumps(row,separators=(',',':'))+'\n')
            require(proc.wait()==0,f'{name}: process exit {proc.returncode}')
        finally:
            timer.cancel()
            if proc.poll() is None:proc.kill()
            proc.wait();proc.stdout.close()
    summary=audit.finish();save(out/'runs'/f'{name}.summary.json',summary);return summary

def run(out,ticks):
    require(not out.exists(),'use a new output directory');require(ticks>=1000 and ticks%1000==0,'horizon must be a multiple of 1000')
    require(bytes_under(ROOT)<25_000_000_000,'workspace cap');out.mkdir(parents=True);(out/'inputs').mkdir();(out/'runs').mkdir()
    cases=[(seed,size,toxic) for seed in (SEEDS if ticks==50000 else SEEDS[:1]) for size in (20,40) for toxic in (False,True)]
    sig=signature();save(out/'software.json',dict(signature=sig,provenance=provenance()));archive_source(out,sig)
    status=dict(status='running',started_at=now(),ticks=ticks,cases=cases,signature=sig);save(out/'execution.json',status);started=time.monotonic();summaries=[]
    try:
        with ThreadPoolExecutor(max_workers=4) as pool:
            futures=[pool.submit(replay,case,ticks,out,started) for case in cases]
            for f in as_completed(futures):
                s=f.result();summaries.append(s);print(f'Validated {len(summaries)}/{len(cases)}: {case_name(s["seed"],s["size"],s["toxic"])} mean={s["final"]["resistance_mean"]}, {time.monotonic()-started:.1f}s',flush=True)
                require(bytes_under(out)<200_000_000,'200MB output cap')
        summaries.sort(key=lambda s:(s['seed'],s['size'],s['toxic']));idx={(s['seed'],s['size'],s['toxic']):s for s in summaries}
        for seed in (SEEDS if ticks==50000 else SEEDS[:1]):
            for size in (20,40):require(idx[(seed,size,False)]['initial_state_sha256']==idx[(seed,size,True)]['initial_state_sha256'],'paired initial state mismatch')
        save(out/'summaries.json',summaries)
        if ticks==50000:save(out/'assessment.json',analyze(summaries))
        require(signature()==sig,'sources changed during execution');require(bytes_under(ROOT)<25_000_000_000,'workspace cap')
        status.update(status='completed',completed_at=now(),elapsed_seconds=time.monotonic()-started,runs=len(summaries),output_sha256={p.relative_to(out).as_posix():sha256(p) for p in out.rglob('*') if p.is_file() and p.name!='execution.json'});save(out/'execution.json',status)
    except Exception as error:
        status.update(status='failed',error=str(error));save(out/'execution.json',status);raise

def audit_saved(out):
    e=json.loads((out/'execution.json').read_text());require(e['status']=='completed','incomplete batch')
    for name,digest in e['output_sha256'].items():require(sha256(out/name)==digest,'saved evidence changed')
    summaries=[]
    for case in e['cases']:
        name=case_name(*case);a=Audit(config_for(*case),e['ticks'])
        with gzip.open(out/'runs'/f'{name}.jsonl.gz','rt',encoding='utf-8') as f:
            for line in f:a.accept(json.loads(line))
        s=a.finish();require(s==json.loads((out/'runs'/f'{name}.summary.json').read_text()),'saved genealogy summary mismatch');summaries.append(s)
    summaries.sort(key=lambda s:(s['seed'],s['size'],s['toxic']));require(summaries==json.loads((out/'summaries.json').read_text()),'saved combined summary mismatch')
    if e['ticks']==50000:require(analyze(summaries)==json.loads((out/'assessment.json').read_text()),'saved analysis mismatch')
    save(out/'saved_log_audit.json',dict(passed=True,runs=len(summaries),execution_sha256=sha256(out/'execution.json')));print('Saved genealogy/analysis audit PASS')
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--ticks',type=int,default=50000);p.add_argument('--audit',action='store_true');a=p.parse_args();(audit_saved(a.out.resolve()) if a.audit else run(a.out.resolve(),a.ticks))
