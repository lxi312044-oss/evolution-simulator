"""Fixed seven-subject foraging-gap observation; no ecological interventions."""
import argparse
from collections import Counter
import gzip
import json
import math
from pathlib import Path
import subprocess
import threading
import time
from diagnose_hp_processes import ROOT,STUDY,BINARY
from diagnose_energy_budget import HP
from run_confirmation import software,provenance,archive_source,save,now,require,bytes_under
from run_resistance_pilot import name_for,digest_json
from run_metadata import sha256
ENERGY=ROOT/'results/energy-budget-diagnosis-20261107-final'

def finite(value):
    if isinstance(value,dict):return all(finite(v) for v in value.values())
    if isinstance(value,list):return all(finite(v) for v in value)
    return not isinstance(value,float) or math.isfinite(value)

def classify(e):
    require(finite(e),'nonfinite foraging observation')
    f=e['feeding'];t=e['target']
    require(e['visible_ne']<=e['visible_n']<=e['visible'],'inconsistent visible counts')
    if e['plant_protected']:return 'plant_protected'
    if e['satiety_blocked']:return 'satiety_blocked'
    require(e['may_feed'],'unexplained feeding gate')
    if t is None:return 'no_target'
    if not t['in_range']:return 'target_out_of_range'
    require(f['attempted'],'missing in-range feeding observation')
    if f['gained_e']>0:return 'positive_income'
    if t['n']<=0:return 'target_no_nutrients'
    if f['eaten_n']<=0:return 'intake_limited'
    if f['available_plant_e']<=0:return 'plant_no_energy'
    if f['energy_capacity']<=0:return 'reserve_full'
    return 'other_zero_income'

class ForagingAudit:
    def __init__(self,windows):
        self.windows={tuple(w['id']):w for w in windows};self.states={}
    def accept(self,e):
        ident=tuple(e['id']);require(ident in self.windows,'unselected subject')
        w=self.windows[ident];s=self.states.setdefault(ident,dict(last_tick=w['start']-1,counts=Counter(),flags=Counter(),records=0,positive_e=0.,eaten_n=0.))
        require(e['tick']==s['last_tick']+1 and e['tick']<=w['end'],'missing/duplicate foraging tick')
        category=classify(e);s['last_tick']=e['tick'];s['records']+=1
        if e['tick']==w['start']:
            require(category=='positive_income','last income anchor not reproduced');return
        require(e['feeding']['gained_e']==0,'unexpected later positive income')
        s['counts'][category]+=1;s['positive_e']+=e['feeding']['gained_e'];s['eaten_n']+=e['feeding']['eaten_n']
        flags=s['flags'];flags['flee']+=int(e['action']=='Flee');flags['energy_limited']+=int(e['affordable_step']+1e-6<e['desired_step'])
        flags['no_visible_plants']+=int(e['visible']==0);flags['no_visible_positive_n_e']+=int(e['visible_ne']==0)
        flags['no_target_despite_visible_n_e']+=int(e['may_feed'] and e['target'] is None and e['visible_ne']>0)
        flags['protected_with_visible_n_e']+=int(e['plant_protected'] and e['visible_ne']>0)
        flags['both_protection_and_satiety']+=int(e['plant_protected'] and e['satiety_blocked'])
    def finish(self):
        require(set(self.states)==set(self.windows),'missing subjects')
        result=[]
        for ident,w in self.windows.items():
            s=self.states[ident];require(s['last_tick']==w['end'] and s['records']==w['end']-w['start']+1,'incomplete foraging window')
            require(sum(s['counts'].values())==w['end']-w['start'],'category partition mismatch')
            result.append(dict(id=list(ident),start=w['start'],end=w['end'],**s))
        return result

def selection():
    result=[]
    prior=json.loads((HP/'selection.json').read_text())
    for arm in json.loads((ENERGY/'assessment.json').read_text()):
        if arm['case'][4]!='on':continue
        windows=[]
        for s in arm['subjects']:
            if s['removal_tick'] is not None and 4001<=s['removal_tick']<=5000 and s['first_hp0'] and s['first_hp0']['phase']=='starvation':
                d=s['first_depletion'];windows.append(dict(id=s['id'],start=d['last_gain'],end=d['tick']))
        plan=next(p for p in prior['planned_cases'] if p['case']==arm['case'])
        result.append(dict(case=arm['case'],windows=windows,hp_watch=plan['watch_file']))
    require(len(result)==4 and sum(len(p['windows']) for p in result)==7,'fixed selection mismatch')
    return result

def verify_inputs():
    for directory in [STUDY,HP,ENERGY]:
        if directory==STUDY:
            manifest=json.loads((directory/'analysis/quality.json').read_text())['input_sha256']
        else:manifest=json.loads((directory/'execution.json').read_text())['output_sha256']
        for name,digest in manifest.items():require(sha256(directory/name)==digest,'input evidence changed')

def run(out):
    require(not out.exists(),'use a new directory');verify_inputs();require(bytes_under(ROOT)<25_000_000_000,'workspace cap')
    plans=selection();out.mkdir(parents=True);save(out/'selection.json',plans)
    sig=software();save(out/'software.json',dict(signature=sig,provenance=provenance()));archive_source(out,sig)
    status=dict(status='running',started_at=now(),signature=sig,input_sha256={str(p.relative_to(ROOT)):sha256(p) for p in [ENERGY/'execution.json',ENERGY/'assessment.json',HP/'execution.json',STUDY/'run.json']})
    save(out/'execution.json',status);start=time.monotonic();summaries=[]
    try:
        for plan in plans:
            case=plan['case'];name=name_for(tuple(case));watch=json.loads((HP/plan['hp_watch']).read_text());watch.update(energy=True,foraging=plan['windows']);save(out/f'{name}.watch.json',watch)
            cmd=[str(BINARY),str(STUDY/'inputs'/f'seed{case[0]}.json'),'A','0','0.2' if case[3]=='low' else '0.8','0.8','6000','100',str(out/f'{name}.watch.json')]
            audit=ForagingAudit(plan['windows']);counts=Counter()
            require(time.monotonic()-start<600,'10 minute cap')
            with gzip.open(STUDY/'runs'/f'{name}.jsonl.gz','rt') as baseline,gzip.open(HP/f'{name}.health.jsonl.gz','rt') as health,gzip.open(ENERGY/f'{name}.energy.jsonl.gz','rt') as energy,gzip.open(out/f'{name}.foraging.jsonl.gz','wt',encoding='utf-8') as log,(out/f'{name}.stderr.log').open('x') as errors:
                proc=subprocess.Popen(cmd,cwd=ROOT,stdout=subprocess.PIPE,stderr=errors,text=True,encoding='utf-8');timer=threading.Timer(min(120,max(.01,600-(time.monotonic()-start))),proc.kill);timer.start()
                try:
                    for line in proc.stdout:
                        row=json.loads(line);kind=row['kind'];counts[kind]+=1
                        if kind=='foraging':audit.accept(row['event']);log.write(json.dumps(row['event'],separators=(',',':'))+'\n')
                        elif kind in ('health','energy'):require(row['event']==json.loads((health if kind=='health' else energy).readline()),f'{kind} changed')
                        else:
                            if kind=='initial':row['initial_background_sha256']=digest_json(row.pop('initial_background'))
                            require(row==json.loads(baseline.readline()),'baseline changed')
                    require(proc.wait()==0,'replay failed');require(not baseline.readline() and not health.readline() and not energy.readline(),'missing original records')
                finally:
                    timer.cancel()
                    if proc.poll() is None:proc.kill()
                    proc.wait();proc.stdout.close()
            summary=dict(case=case,counts=dict(counts),subjects=audit.finish());save(out/f'{name}.summary.json',summary);summaries.append(summary)
            require(software()==sig,'execution sources changed');require(bytes_under(out)<50_000_000,'50MB cap')
            print(name,summary['subjects'],flush=True)
        save(out/'assessment.json',summaries);verify_inputs()
        status.update(status='completed',completed_at=now(),elapsed_seconds=time.monotonic()-start,baseline_equal=True,health_equal=True,energy_equal=True,output_sha256={p.name:sha256(p) for p in out.iterdir() if p.is_file() and p.name!='execution.json'})
        save(out/'execution.json',status)
    except Exception as error:
        status.update(status='failed',error=str(error));save(out/'execution.json',status);raise

def audit_saved(out):
    execution=json.loads((out/'execution.json').read_text());require(execution['status']=='completed','incomplete execution')
    for name,digest in execution['output_sha256'].items():require(sha256(out/name)==digest,'saved evidence changed')
    for name,digest in execution['input_sha256'].items():require(sha256(ROOT/name)==digest,'input changed')
    plans=json.loads((out/'selection.json').read_text());expected=json.loads((out/'assessment.json').read_text());n=0
    for plan,summary in zip(plans,expected,strict=True):
        require(plan['case']==summary['case'],'case mismatch');audit=ForagingAudit(plan['windows'])
        with gzip.open(out/f'{name_for(tuple(plan["case"]))}.foraging.jsonl.gz','rt',encoding='utf-8') as f:
            for line in f:audit.accept(json.loads(line));n+=1
        require(audit.finish()==summary['subjects'],'saved summary mismatch')
    save(out/'saved_log_audit.json',dict(passed=True,records=n,subjects=sum(len(p['windows']) for p in plans),execution_sha256=sha256(out/'execution.json')))
    print('saved audit PASS',n)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--audit',action='store_true');args=p.parse_args();(audit_saved if args.audit else run)(args.out.resolve())
