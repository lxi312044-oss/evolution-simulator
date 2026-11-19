"""Fixed 32-seed factorial comparison of protection and soil germination."""
import argparse
from concurrent.futures import ThreadPoolExecutor,as_completed
import gzip
import json
import math
from pathlib import Path
import random
import subprocess
import time
from run_confirmation import ROOT,software,provenance,archive_source,save,now,require,bytes_under
from run_metadata import sha256
from diagnose_hp_processes import STUDY
BINARY=ROOT/'target/release/examples/plant_protection_comparison.exe'

def signature():
    s=software();s['experiment_binary_sha256']=sha256(BINARY);return s

def validate(row,seed,level,protection,germination):
    def finite(x):
        if isinstance(x,dict):return all(finite(v) for v in x.values())
        if isinstance(x,list):return all(finite(v) for v in x)
        return not isinstance(x,float) or math.isfinite(x)
    require(finite(row),'nonfinite data');require(row['seed']==seed and row['ticks']==6000 and row['interval']==100,'identity/horizon mismatch')
    require(abs(row['resistance']-(.2 if level=='low' else .8))<1e-6 and row['protection']==protection and row['germination']==germination,'arm mismatch')
    require([s['tick'] for s in row['samples']]==list(range(100,6001,100)),'snapshot gap')
    c=row['effective_config']
    with gzip.open(STUDY/'runs'/f'seed{seed}_A_type0_{level}_on.jsonl.gz','rt') as stream:expected=json.loads(stream.readline())['effective_config']
    expected['herbivores']['plant_protection_enabled']=protection
    require(c==expected,'unexpected effective config difference')
    # Effective controls are deliberately the same as pilot.prepare, except protection.
    require(c['herbivores']['plant_protection_enabled']==protection,'protection config mismatch')
    require(c['toxin']['plant_toxin_type_mutation_probability']==0 and c['toxin']['resistance_mutation_probability']==0 and c['toxin']['initial_type0_fraction']==1 and c['toxin']['resistance_effectiveness']==.8,'toxin controls mismatch')
    require(row['max_n_error']<=1e-3 and row['max_e_error']<=1e-3,'conservation audit failed')
    require(0<=row['min_plants']<=300 and 0<=row['zero_plant_ticks']<=6000,'invalid plant census')
    if germination:require(row['germination_parameters']==dict(soil_threshold=20,probability_per_cell=.02,check_interval=20,max_births_per_check=20),'germination parameters mismatch')
    if not germination:require(row['germination_births']==0 and row['germination_parameters'] is None,'germination despite OFF')
    return c

def metric(row):
    s=row['samples'][-1];d=row['diagnostics']
    return dict(plants=s['plants'],min_plants=row['min_plants'],zero_plant_ticks=row['zero_plant_ticks'],
        germination_births=row['germination_births'],herbivores=s['herbivores_hp_positive'],herbivore_births=d['herbivore_births'],
        hp_deaths=d['herbivore_hp_deaths'],age_deaths=d['herbivore_age_deaths'],starvation_ticks=d['phase21_starvation_damage_ticks'],
        starvation_damage=d['phase21_starvation_hp_damage'],protected_ticks=d['herbivore_plant_protection_blocked_ticks'],
        focal_births=row['focal_births'],lineage_births=row['lineage_births'],lineage_survivors=row['lineage_positive_survivors'])

def analyze(rows):
    groups=[];contrasts=[]
    for g in (False,True):
        for p in (False,True):
            subset=[r for r in rows if r['germination']==g and r['protection']==p]
            groups.append(dict(germination=g,protection=p,runs=len(subset),independent_seeds=32,
                means={k:sum(metric(r)[k] for r in subset)/len(subset) for k in metric(subset[0])},
                zero_worlds=sum(r['zero_plant_ticks']>0 for r in subset),unrecovered_zero_worlds=sum(any(e['recovery'] is None for e in r['zero_episodes']) for r in subset),
                min_observed_plants=min(r['min_plants'] for r in subset)))
        indexed={(r['seed'],'low' if r['resistance']<.5 else 'high',r['protection']):metric(r) for r in rows if r['germination']==g}
        for key in metric(rows[0]):
            diffs=[sum(indexed[(seed,level,False)][key]-indexed[(seed,level,True)][key] for level in ('low','high'))/2 for seed in range(34501,34533)]
            rng=random.Random(20261107);boots=sorted(sum(rng.choice(diffs) for _ in diffs)/32 for _ in range(2000))
            contrasts.append(dict(germination=g,metric=key,estimand='protection_OFF_minus_ON, equal-weight within seed over low/high',independent_seeds=32,mean=sum(diffs)/32,ci95=[boots[49],boots[1949]],negative_seeds=sum(d<0 for d in diffs),positive_seeds=sum(d>0 for d in diffs)))
    return dict(groups=groups,protection_contrasts=contrasts,max_n_error=max(r['max_n_error'] for r in rows),max_e_error=max(r['max_e_error'] for r in rows),
        recovered_zero_episodes=sum(e['recovery'] is not None for r in rows for e in r['zero_episodes']))

def check_baseline(row):
    seed=row['seed'];level='low' if row['resistance']<.5 else 'high'
    with gzip.open(STUDY/'runs'/f'seed{seed}_A_type0_{level}_on.jsonl.gz','rt') as f:
        for line in f:
            old=json.loads(line)
            if old['kind']=='initial':initial=old
            if old['kind']=='snapshot':snapshot=old
    legacy=old['legacy_result'];require(row['final_rng']==old['final_rng'] and row['final_ecology']==legacy['final_ecology'],'old baseline RNG/ecology changed')
    from run_resistance_pilot import digest_json
    require(digest_json(row['initial_background'])==initial['initial_background_sha256'] and row['initial']==initial['initial'] and row['effective_config']==initial['effective_config'],'old initial changed')
    require(row['focal_births']==legacy['outcome']['births'] and row['lineage_births']==snapshot['metrics']['lineage_births'] and row['lineage_removals']==snapshot['metrics']['removed_descendants'] and row['lineage_positive_survivors']==snapshot['metrics']['living_descendants'],'old lineage changed')

def run(out):
    require(not out.exists(),'use a new output directory');require(bytes_under(ROOT)<25_000_000_000,'workspace cap')
    quality=json.loads((STUDY/'analysis/quality.json').read_text())
    for name,digest in quality['input_sha256'].items():require(sha256(STUDY/name)==digest,'baseline evidence changed')
    out.mkdir(parents=True);(out/'runs').mkdir();sig=signature();save(out/'software.json',dict(signature=sig,provenance=provenance()));archive_source(out,sig)
    start=time.monotonic();status=dict(status='running',started_at=now(),signature=sig,quality_sha256=sha256(STUDY/'analysis/quality.json'));save(out/'execution.json',status)
    plans=[(seed,level,p,g) for seed in range(34501,34533) for level in ('low','high') for p in (True,False) for g in (False,True)]
    save(out/'plan.json',dict(cases=plans,ticks=6000,interval=100,workers=4,germination=dict(soil_threshold=20,probability_per_cell=.02,check_interval=20,max_births_per_check=20)))
    def execute(case):
        seed,level,p,g=case;name=f'seed{seed}_{level}_p{int(p)}_g{int(g)}'
        remaining=600-(time.monotonic()-start);require(remaining>0,'10-minute cap')
        cmd=[str(BINARY),str(STUDY/'inputs'/f'seed{seed}.json'),'.2' if level=='low' else '.8',str(p).lower(),str(g).lower()]
        result=subprocess.run(cmd,cwd=ROOT,capture_output=True,text=True,encoding='utf-8',timeout=min(120,remaining))
        (out/'runs'/f'{name}.stderr.log').write_text(result.stderr,encoding='utf-8');require(result.returncode==0,f'{name} exit {result.returncode}: {result.stderr[-400:]}')
        row=json.loads(result.stdout);validate(row,*case)
        if p and not g:check_baseline(row)
        save(out/'runs'/f'{name}.json',row)
        return row
    try:
        rows=[]
        with ThreadPoolExecutor(max_workers=4) as pool:
            futures=[pool.submit(execute,c) for c in plans]
            for f in as_completed(futures):
                rows.append(f.result())
                if len(rows)%16==0:print(f'Validated {len(rows)}/256; {time.monotonic()-start:.1f}s',flush=True)
                require(bytes_under(out)<100_000_000,'100MB cap')
        rows.sort(key=lambda r:(r['seed'],r['resistance'],r['protection'],r['germination']))
        # All arms start from the same actual world state (protection/config flag is the only difference).
        from run_resistance_pilot import digest_json
        for seed in range(34501,34533):
            for level in ('low','high'):
                subset=[r for r in rows if r['seed']==seed and (r['resistance']<.5)==(level=='low')]
                require(len({digest_json(r['initial_background']) for r in subset})==1 and all(r['initial']==subset[0]['initial'] for r in subset),'initial paired state mismatch')
                configs=[json.loads(json.dumps(r['effective_config'])) for r in subset]
                for c in configs:c['herbivores']['plant_protection_enabled']=True
                require(all(c==configs[0] for c in configs),'unintended config differences')
        save(out/'assessment.json',analyze(rows));require(signature()==sig,'sources changed during experiment')
        status.update(status='completed',completed_at=now(),runs=len(rows),baseline_matches=64,elapsed_seconds=time.monotonic()-start,output_sha256={p.relative_to(out).as_posix():sha256(p) for p in out.rglob('*') if p.is_file() and p.name!='execution.json'})
        save(out/'execution.json',status)
    except Exception as error:
        status.update(status='failed',error=str(error));save(out/'execution.json',status);raise

def audit(out):
    e=json.loads((out/'execution.json').read_text());require(e['status']=='completed' and e['runs']==256 and e['baseline_matches']==64,'incomplete study')
    for name,digest in e['output_sha256'].items():require(sha256(out/name)==digest,'saved output changed')
    require(sha256(STUDY/'analysis/quality.json')==e['quality_sha256'],'baseline quality changed')
    quality=json.loads((STUDY/'analysis/quality.json').read_text())
    for name,digest in quality['input_sha256'].items():require(sha256(STUDY/name)==digest,'baseline input changed')
    rows=[]
    for seed,level,p,g in json.loads((out/'plan.json').read_text())['cases']:
        row=json.loads((out/'runs'/f'seed{seed}_{level}_p{int(p)}_g{int(g)}.json').read_text());validate(row,seed,level,p,g);rows.append(row)
        if p and not g:check_baseline(row)
    rows.sort(key=lambda r:(r['seed'],r['resistance'],r['protection'],r['germination']))
    require(analyze(rows)==json.loads((out/'assessment.json').read_text()),'saved assessment mismatch')
    analysis_sig=signature();analysis_dir=out/('analysis-source-'+analysis_sig['manifest_sha256'][:12]);analysis_dir.mkdir(exist_ok=True)
    save(analysis_dir/'software.json',dict(signature=analysis_sig,provenance=provenance()));archive_source(analysis_dir,analysis_sig)
    save(out/('saved_log_audit-'+analysis_sig['manifest_sha256'][:12]+'.json'),dict(analysis_manifest_sha256=analysis_sig['manifest_sha256'],strict_absolute_conservation_tolerance=1e-3,passed=True,runs=256,independent_seeds=32,baseline_matches=64,execution_sha256=sha256(out/'execution.json')));print('saved audit PASS')
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--audit',action='store_true');a=p.parse_args();(audit if a.audit else run)(a.out.resolve())
