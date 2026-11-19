"""Audit exact reserve transactions in the fixed HP-study subjects without changing ecology."""
import argparse
import csv
from collections import Counter
import gzip
import json
import math
from pathlib import Path
import subprocess
import threading
import time
from diagnose_hp_processes import ROOT, STUDY, BINARY
from run_confirmation import software, provenance, archive_source, save, now, require, bytes_under
from run_resistance_pilot import name_for, digest_json
from run_metadata import sha256
HP=ROOT/'results/hp-process-diagnosis-20261107'

class EnergyAudit:
    def __init__(self,members,hp):
        self.members={tuple(m['id']):m for m in members}
        self.hp={tuple(m['id']):m for m in hp}
        self.states={};self.seq=0;self.max_residual=0.
    def accept(self,e):
        self.seq+=1
        require(e['seq']==self.seq,'energy sequence gap')
        ident=tuple(e['id']);require(ident in self.members,'unselected subject')
        b,a,d=(e[k] for k in ('before_e','after_e','delta_e'))
        require(all(math.isfinite(v) for v in (b,a,d)) and min(a,b)>=-1e-9,'invalid energy')
        phase=e['phase'];tick=e['tick']
        require(phase in ('birth','feeding','basal','movement','resistance_upkeep','offspring_transfer','reproduction_work','predation_bite','predation_lethal_meal','removal_heat','removal','end_tick'),'unknown flow')
        if phase=='birth':
            require(ident not in self.states and tick==self.members[ident]['birth_tick'] and b==0 and d>=0,'invalid birth')
            self.states[ident]=dict(balance=0.,total=Counter(),last_tick=tick,closed=False,first_depletion=None,
                accumulated=0.,checkpoint_tick=tick-1,depletion_episodes=0,last_gain=None,first_hp0=self.hp[ident]['first_hp0'],events=0)
        require(ident in self.states,'missing birth allocation')
        s=self.states[ident]
        require(not s['closed'] and tick>=s['last_tick'],'event after closure or reversed time')
        s['accumulated']+=d
        residual=max(abs(s['balance']-b),abs(b+d-a),abs(s['accumulated']-a))
        self.max_residual=max(self.max_residual,residual)
        require(residual<=1e-8+1e-10*max(abs(b),abs(a),abs(s['balance'])),'unaccounted reserve flow')
        require((phase in ('birth','feeding') and d>=0) or (phase not in ('birth','feeding') and d<=0),'wrong flow direction')
        if phase in ('end_tick','removal'):require(d==0 and b==a,'invalid checkpoint')
        s['total'][phase]+=d;s['balance']=a;s['last_tick']=tick;s['events']+=1
        if phase=='feeding' and d>0:s['last_gain']=tick
        cross=b>1e-9 and a<=1e-9
        if cross:
            s['depletion_episodes']+=1
            h=s['first_hp0']
            before_hp0=h is None or tick<h['tick'] or (tick==h['tick'] and e['health_seq']<h['seq'])
            point=dict(tick=tick,phase=phase,before_e=b,after_e=a,before_first_hp0=before_hp0,
                last_gain=s['last_gain'],budget=dict(s['total']))
            if s['first_depletion'] is None:s['first_depletion']=point
        if phase=='end_tick':
            require(tick==s['checkpoint_tick']+1,'missing per-tick checkpoint')
            s['checkpoint_tick']=tick
        if phase=='removal':
            require(tick==s['checkpoint_tick']+1,'missing pre-removal checkpoint')
            r=self.members[ident]['removal']
            require(r is not None and tick==r['tick'] and abs(a)<=1e-9,'invalid removal')
            s['closed']=True
    def finish(self):
        require(set(self.states)==set(self.members),'missing subjects')
        out=[]
        for ident,s in self.states.items():
            m=self.members[ident];require(s['closed']==(m['removal'] is not None),'missing closure')
            if not s['closed']:require(s['checkpoint_tick']==6000,'missing final checkpoint')
            out.append(dict(id=list(ident),birth_tick=m['birth_tick'],removal_tick=None if m['removal'] is None else m['removal']['tick'],**s))
        return out

def run(out):
    require(not out.exists(),'use a new output directory')
    selection=json.loads((HP/'selection.json').read_text())
    prior=json.loads((HP/'execution.json').read_text());require(prior['status']=='completed','HP study incomplete')
    for name,digest in prior['output_sha256'].items():require(sha256(HP/name)==digest,'HP evidence changed')
    quality=json.loads((STUDY/'analysis/quality.json').read_text())
    for name,digest in quality['input_sha256'].items():require(sha256(STUDY/name)==digest,'confirmation evidence changed')
    require(bytes_under(ROOT)<25_000_000_000,'workspace limit')
    out.mkdir(parents=True);sig=software();save(out/'software.json',dict(signature=sig,provenance=provenance()));archive_source(out,sig)
    status=dict(status='running',started_at=now(),selection_sha256=sha256(HP/'selection.json'),hp_execution_sha256=sha256(HP/'execution.json'),signature=sig)
    save(out/'execution.json',status);start=time.monotonic();summaries=[]
    try:
        for plan in selection['planned_cases']:
            case=tuple(plan['case']);name=name_for(case)
            watch=json.loads((HP/plan['watch_file']).read_text());watch['energy']=True;save(out/f'{name}.watch.json',watch)
            hp_summary=json.loads((HP/f'{name}.summary.json').read_text())
            audit=EnergyAudit(plan['members'],hp_summary['subjects'])
            cmd=[str(BINARY),str(STUDY/'inputs'/f'seed{case[0]}.json'),'A','0','0.2' if case[3]=='low' else '0.8','0' if case[4]=='off' else '0.8','6000','100',str(out/f'{name}.watch.json')]
            count=Counter();require(time.monotonic()-start<600,'10 minute cap')
            with (out/f'{name}.stderr.log').open('x',encoding='utf-8') as errors, gzip.open(STUDY/'runs'/f'{name}.jsonl.gz','rt') as baseline, gzip.open(HP/f'{name}.health.jsonl.gz','rt') as health, gzip.open(out/f'{name}.energy.jsonl.gz','wt',encoding='utf-8') as energy:
                proc=subprocess.Popen(cmd,cwd=ROOT,stdout=subprocess.PIPE,stderr=errors,text=True,encoding='utf-8')
                timer=threading.Timer(min(120,max(.01,600-(time.monotonic()-start))),proc.kill);timer.start()
                try:
                    for line in proc.stdout:
                        row=json.loads(line);kind=row['kind'];count[kind]+=1
                        if kind=='energy':
                            audit.accept(row['event']);energy.write(json.dumps(row['event'],separators=(',',':'))+'\n')
                        elif kind=='health':require(row['event']==json.loads(health.readline()),'HP trajectory changed')
                        else:
                            if kind=='initial':row['initial_background_sha256']=digest_json(row.pop('initial_background'))
                            require(row==json.loads(baseline.readline()),'baseline trajectory changed')
                    require(proc.wait()==0,'replay failed');require(not baseline.readline() and not health.readline(),'missing baseline/HP records')
                finally:
                    timer.cancel()
                    if proc.poll() is None:proc.kill()
                    proc.wait();proc.stdout.close()
            subjects=audit.finish();summary=dict(case=list(case),counts=dict(count),max_residual=audit.max_residual,subjects=subjects)
            save(out/f'{name}.summary.json',summary);summaries.append(summary)
            require(bytes_under(out)<50_000_000,'50 MB output cap');require(software()==sig,'sources changed during execution')
            print(f'{name}: {len(subjects)} subjects; {count["energy"]} energy records; residual {audit.max_residual:.3g}',flush=True)
        save(out/'assessment.json',summaries)
        status.update(status='completed',completed_at=now(),elapsed_seconds=time.monotonic()-start,baseline_equal=True,health_equal=True,replays=len(summaries),output_sha256={p.name:sha256(p) for p in sorted(out.iterdir()) if p.is_file() and p.name!='execution.json'})
        save(out/'execution.json',status)
    except Exception as error:
        status.update(status='failed',error=str(error));save(out/'execution.json',status);raise
def audit_saved(out):
    execution=json.loads((out/'execution.json').read_text())
    require(execution['status']=='completed','execution incomplete')
    for name,digest in execution['output_sha256'].items():require(sha256(out/name)==digest,'saved output changed')
    selection=json.loads((HP/'selection.json').read_text())
    require(sha256(HP/'selection.json')==execution['selection_sha256'],'selection changed')
    assessment=json.loads((out/'assessment.json').read_text());rows=[];records=0;residual=0.
    for plan,expected in zip(selection['planned_cases'],assessment,strict=True):
        name=name_for(tuple(plan['case']));hp=json.loads((HP/f'{name}.summary.json').read_text())
        audit=EnergyAudit(plan['members'],hp['subjects'])
        with gzip.open(out/f'{name}.energy.jsonl.gz','rt',encoding='utf-8') as stream:
            for line in stream:audit.accept(json.loads(line));records+=1
        subjects=audit.finish();require(subjects==expected['subjects'],'saved ledger summary mismatch')
        residual=max(residual,audit.max_residual)
        for subject in subjects:
            point=subject['first_depletion'];h=subject['first_hp0'];budget={} if point is None else point['budget']
            rows.append(dict(case=name,id=str(subject['id']),birth_tick=subject['birth_tick'],removal_tick=subject['removal_tick'],
                first_hp0_phase=None if h is None else h['phase'],first_hp0_tick=None if h is None else h['tick'],
                first_depletion_tick=None if point is None else point['tick'],first_depletion_phase=None if point is None else point['phase'],
                before_first_hp0=None if point is None else point['before_first_hp0'],
                last_gain_before_depletion=None if point is None else point['last_gain'],depletion_episodes=subject['depletion_episodes'],
                **{k:budget.get(k,0.) for k in ('birth','feeding','basal','movement','resistance_upkeep','offspring_transfer','reproduction_work','predation_bite','predation_lethal_meal','removal_heat')}))
    with (out/'individual_budgets.csv').open('x',newline='',encoding='utf-8') as f:
        writer=csv.DictWriter(f,fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)
    report=dict(passed=True,individuals=len(rows),energy_records=records,max_prefix_residual=residual,
        execution_sha256=sha256(out/'execution.json'),assessment_sha256=sha256(out/'assessment.json'),
        csv_sha256=sha256(out/'individual_budgets.csv'))
    save(out/'saved_log_audit.json',report);print(json.dumps(report))

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--audit',action='store_true')
    args=p.parse_args();(audit_saved if args.audit else run)(args.out.resolve())
