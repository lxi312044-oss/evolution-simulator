"""Preselected old-seed replays with exact HP-write observations, not counterfactual death causes."""
import argparse
from collections import Counter
import gzip
import json
import math
from pathlib import Path
import subprocess
import threading
import time

from confirmation_metrics import entity
from run_confirmation import LogValidator, software, provenance, archive_source, save, now, require, bytes_under
from run_resistance_pilot import name_for, digest_json
from run_metadata import sha256

ROOT=Path(__file__).resolve().parents[1]
STUDY=ROOT/'results/confirmation-study-20261107'
BINARY=ROOT/'target/release/examples/resistance_confirmation.exe'


def rank_cases(scores, count=2):
    candidates=[x for x in scores if x['birth_difference']>0 and x['survival_difference']<0]
    fallback=not candidates
    if fallback:candidates=[x for x in scores if x['window_removal_difference']>0]
    return sorted(candidates,key=lambda x:(-x['window_removal_difference'],x['seed']))[:count],fallback


def source_case(case):
    path=STUDY/'runs'/f'{name_for(case)}.jsonl.gz'
    rows=[]
    with gzip.open(path,'rt',encoding='utf-8') as stream:
        for line in stream:rows.append(json.loads(line))
    initial=rows[0]
    depths={entity(initial['focal_id']):0}
    members={}
    for row in rows:
        if row['kind']=='birth' and entity(row['parent']) in depths:
            depth=depths[entity(row['parent'])]+1
            depths[entity(row['child'])]=depth
            if depth in [1,2]:
                members[entity(row['child'])]=dict(id=row['child'],parent=row['parent'],
                    birth_tick=row['tick'],depth=depth,resistance=row['resistance'],removal=None)
        elif row['kind']=='removal' and entity(row['id']) in members:
            members[entity(row['id'])]['removal']=row
    result=json.loads((STUDY/'runs'/f'{name_for(case)}.result.json').read_text())
    return rows,members,result


def select(out):
    require(not out.exists(),'use new diagnostic directory')
    quality=json.loads((STUDY/'analysis/quality.json').read_text())
    require(quality['passed'] and quality['arms']==768,'source study incomplete')
    for name,digest in quality['input_sha256'].items():
        require(sha256(STUDY/name)==digest,'source evidence changed')
    scores=[]
    for seed in range(34501,34533):
        values={}
        for level in ['low','high']:
            _,members,result=source_case((seed,'A',0,level,'on'))
            values[level]=(result['endpoints']['6000'],sum(x['removal'] is not None
                and 4001<=x['removal']['tick']<=5000 for x in members.values()))
        low,high=values['low'],values['high']
        scores.append(dict(seed=seed,birth_difference=high[0]['lineage_births']-low[0]['lineage_births'],
            survival_difference=high[0]['living_descendants']-low[0]['living_descendants'],
            window_removal_difference=high[1]-low[1]))
    chosen,fallback=rank_cases(scores)
    require(len(chosen)>0,'no cases meet fixed selection rule')
    out.mkdir(parents=True)
    planned=[]
    for item in chosen:
        for level in ['low','high']:
            for defense in ['off','on']:
                case=(item['seed'],'A',0,level,defense)
                _,members,_=source_case(case)
                name=name_for(case)
                health=dict(watched=[list(x) for x in sorted(members)],window=[3500,5100])
                save(out/f'{name}.watch.json',health)
                planned.append(dict(case=list(case),watch_file=f'{name}.watch.json',
                    watch_sha256=sha256(out/f'{name}.watch.json'),members=list(members.values()),
                    source_log_sha256=sha256(STUDY/'runs'/f'{name}.jsonl.gz')))
    selection=dict(selected_seeds=[x['seed'] for x in chosen],scores=scores,fallback=fallback,
        rule='birth_delta>0 & survival_delta<0; window child/grandchild removal delta descending, seed ascending; top2',
        ticks=6000,window=[3500,5100],planned_cases=planned,
        study_run_sha256=sha256(STUDY/'run.json'),study_quality_sha256=sha256(STUDY/'analysis/quality.json'))
    save(out/'selection.json',selection)
    print(json.dumps(dict(selected=chosen,fallback=fallback,arms=len(planned))))


class HealthAudit:
    def __init__(self,members):
        self.members={entity(x['id']):x for x in members}
        self.sequence=0
        self.states={}

    def accept(self,e):
        require(e['seq']==self.sequence+1,'health event sequence gap')
        self.sequence=e['seq']
        ident=entity(e['id'])
        require(ident in self.members,'unselected individual logged')
        for key in ['hp_after','reserve_e','reserve_n','stomach_e','stomach_n','body_n','toxin_damage','upkeep_paid']:
            require(type(e[key]) in [int,float] and math.isfinite(e[key]),f'invalid health numeric {key}')
        after,before=e['hp_after'],e['hp_before']
        if ident not in self.states:
            require(e['phase']=='birth' and e['tick']==self.members[ident]['birth_tick'] and before is None,
                    'missing birth health state')
            require(after>0,'initial nonpositive newborn HP is outside classified HP-write paths')
            self.states[ident]=dict(initial_hp=after,last_hp=after,damage=Counter(),nominal_damage=Counter(),
                hp_increases=0.,crossings=[],recoveries=[],first_prior_damage=None,terminal=None,
                feeding_events=0,energy_gained=0.,last_energy_gain_tick=None,zero_reserve_ticks=set(),
                starvation_ticks=set(),upkeep_zero_ticks=set(),last_upkeep=0.)
        state=self.states[ident]
        if before is None:
            require(after==state['last_hp'],'HP changed without an observed update')
        else:
            require(type(before) in [int,float] and math.isfinite(before),'invalid hp_before')
            require(before==state['last_hp'],'HP transition continuity gap')
            crossed=before>0 and after<=0
            recovered=before<=0 and after>0
            require(e['crossed_hp0']==crossed and e['recovered_positive']==recovered,'invalid crossing flags')
            if crossed:
                if not state['crossings']:
                    state['first_prior_damage']=dict(state['damage'])
                    state['last_energy_gain_before_first_cross']=state['last_energy_gain_tick']
                state['crossings'].append(e)
            if recovered:state['recoveries'].append(e)
            if before>after:state['damage'][e['phase']]+=before-after
            elif after>before:state['hp_increases']+=after-before
            state['nominal_damage'][e['phase']]+=e['details'].get('damage',0)
        state['last_hp']=after
        state['last_upkeep']=e['upkeep_paid']
        if e['reserve_e']<=1e-9:state['zero_reserve_ticks'].add(e['tick'])
        if e['phase']=='starvation':state['starvation_ticks'].add(e['tick'])
        if e['phase']=='resistance_upkeep' and e['details']['reserve_before']>1e-9 and e['reserve_e']<=1e-9:
            state['upkeep_zero_ticks'].add(e['tick'])
        if e['phase']=='feeding' and e['details']['gained_e']>0:
            state['feeding_events']+=1;state['energy_gained']+=e['details']['gained_e']
            state['last_energy_gain_tick']=e['tick']
        if e['phase']=='pre_removal':
            require(state['terminal'] is None,'duplicate health removal')
            state['terminal']=e

    def finish(self,present):
        require(set(self.states)==set(self.members),'missing watched individual history')
        endpoint={entity(x['id']):x['hp'] for x in present}
        result=[]
        for ident,member in self.members.items():
            state=self.states[ident]
            removal=member['removal'];terminal=state['terminal']
            require((removal is None)==(terminal is None),'health/source removal coverage mismatch')
            if removal:
                require(terminal['tick']==removal['tick'] and terminal['hp_after']==removal['hp'], 'removal state mismatch')
                if removal['hp']<=0:require(bool(state['crossings']),'HP0 removal lacks a crossing')
            else:
                require(ident in endpoint and endpoint[ident]==state['last_hp'],'survivor HP history coverage mismatch')
            require(abs(state['initial_hp']-sum(state['damage'].values())+state['hp_increases']-state['last_hp'])<1e-5,
                    'HP accounting mismatch')
            first=state['crossings'][0] if state['crossings'] else None
            if first:require(first['phase'] in ['toxin','starvation','predation'],'unexpected first HP0 phase')
            through_first=None
            if first:
                through_first=Counter(state['first_prior_damage'])
                through_first[first['phase']]+=max(first['hp_before']-first['hp_after'],0)
            result.append(dict(id=list(ident),parent=member['parent'],depth=member['depth'],birth_tick=member['birth_tick'],
                removal_tick=None if removal is None else removal['tick'],
                removal_trigger=None if removal is None else removal['trigger'],
                initial_hp=state['initial_hp'],last_hp=state['last_hp'],first_hp0=first,
                first_prior_hp_damage=state['first_prior_damage'],hp_damage=dict(state['damage']),
                hp_damage_through_first_hp0=None if through_first is None else dict(through_first),
                nominal_hp_damage=dict(state['nominal_damage']),hp_increases=state['hp_increases'],
                crossing_count=len(state['crossings']),positive_recovery_count=len(state['recoveries']),
                observed_zero_reserve_ticks=len(state['zero_reserve_ticks']),starvation_ticks=len(state['starvation_ticks']),
                upkeep_tipped_zero_ticks=len(state['upkeep_zero_ticks']),
                first_hp0_upkeep_tipped_zero=bool(first and first['tick'] in state['upkeep_zero_ticks']),
                first_hp0_after_energy_gain_tick=state.get('last_energy_gain_before_first_cross'),
                feeding_events=state['feeding_events'],energy_gained=state['energy_gained'],upkeep_paid=state['last_upkeep']))
        return result


def run(out):
    selection=json.loads((out/'selection.json').read_text())
    require(sha256(STUDY/'run.json')==selection['study_run_sha256'],'source study changed')
    require(sha256(STUDY/'analysis/quality.json')==selection['study_quality_sha256'],'source quality changed')
    require(bytes_under(ROOT)<25_000_000_000,'workspace limit')
    require(not (out/'execution.json').exists(),'refuse replay overwrite')
    sig=software()
    save(out/'software.json',dict(signature=sig,provenance=provenance()))
    archive_source(out,sig)
    status=dict(status='running',started_at=now(),selection_sha256=sha256(out/'selection.json'),signature=sig)
    save(out/'execution.json',status)
    started=time.monotonic()
    summaries=[]
    try:
        for planned in selection['planned_cases']:
            require(time.monotonic()-started<600,'10-minute replay cap')
            require(sha256(out/planned['watch_file'])==planned['watch_sha256'],'selection targets changed')
            case=tuple(planned['case']);name=name_for(case)
            original=STUDY/'runs'/f'{name}.jsonl.gz'
            require(sha256(original)==planned['source_log_sha256'],'original log changed')
            config=STUDY/'inputs'/f'seed{case[0]}.json'
            cmd=[str(BINARY),str(config),'A','0','0.2' if case[3]=='low' else '0.8',
                 '0' if case[4]=='off' else '0.8','6000','100',str(out/planned['watch_file'])]
            audit=HealthAudit(planned['members']);validator=LogValidator(case,6000)
            proc=None;timer=None;health_events=0;baseline_records=0
            with (out/f'{name}.stderr.log').open('x',encoding='utf-8') as errors:
                proc=subprocess.Popen(cmd,cwd=ROOT,stdout=subprocess.PIPE,stderr=errors,text=True,encoding='utf-8')
                timer=threading.Timer(min(120,max(.01,600-(time.monotonic()-started))),proc.kill)
                timer.start()
                try:
                    with gzip.open(original,'rt',encoding='utf-8') as source, (out/f'{name}.health.jsonl.gz').open('xb') as raw:
                        with gzip.GzipFile(filename='',fileobj=raw,mode='wb',mtime=0) as trace:
                            for line in proc.stdout:
                                row=json.loads(line)
                                if row['kind']=='health':
                                    audit.accept(row['event'])
                                    trace.write((json.dumps(row['event'],separators=(',',':'))+'\n').encode())
                                    health_events+=1
                                else:
                                    if row['kind']=='initial':row['initial_background_sha256']=digest_json(row.pop('initial_background'))
                                    expected=source.readline()
                                    require(bool(expected) and row==json.loads(expected),f'baseline trajectory changed: {name}')
                                    validator.accept(row);baseline_records+=1
                            require(not source.readline(),'baseline records missing')
                    require(proc.wait()==0,f'replay exit {proc.returncode}')
                finally:
                    timer.cancel()
                    if proc.poll() is None:proc.kill()
                    proc.wait();proc.stdout.close()
            validator.finish()
            subjects=audit.finish(validator.snapshots[-1]['present'])
            summary=dict(case=list(case),baseline_records=baseline_records,health_events=health_events,
                         subjects=subjects,health_log_sha256=sha256(out/f'{name}.health.jsonl.gz'))
            save(out/f'{name}.summary.json',summary);summaries.append(summary)
            require(bytes_under(out)<50_000_000,'50MB log cap')
            require(software()==sig,'execution sources changed')
            print(f'Validated {name}: {len(subjects)} subjects, {health_events} events',flush=True)
        save(out/'assessment.json',summaries)
        require(bytes_under(ROOT)<25_000_000_000,'workspace limit')
        status.update(status='completed',completed_at=now(),elapsed_seconds=time.monotonic()-started,
            replay_count=len(summaries),baseline_equal=True,
            output_sha256={p.name:sha256(p) for p in sorted(out.iterdir()) if p.is_file() and p.name!='execution.json'})
        save(out/'execution.json',status)
        print(json.dumps(dict(status='completed',replays=len(summaries),elapsed_seconds=status['elapsed_seconds'])))
    except Exception as error:
        status.update(status='failed',error=str(error),failed_at=now())
        save(out/'execution.json',status)
        raise


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('stage',choices=['select','run'])
    parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args()
    (select if args.stage=='select' else run)(args.out.resolve())
