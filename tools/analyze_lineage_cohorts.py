"""Issue #5 exploratory decomposition of existing confirmation logs; no simulation."""
import argparse
from collections import Counter, defaultdict
import csv
import gzip
import json
from pathlib import Path
import time

from confirmation_metrics import entity, lineage_metrics
from run_confirmation import cases, load_run, require
from run_metadata import command, sha256
from run_resistance_pilot import name_for, digest_json

ROOT = Path(__file__).resolve().parents[1]
COUNTS = ('births', 'removed', 'positive_hp_survivors', 'pending_hp0',
          'eligible_500', 'unremoved_at_500', 'late_births', 'late_observed_removals',
          'late_censored_present', 'observed_unremoved_ticks')


def zero():
    return dict.fromkeys(COUNTS, 0)


def decompose(founders, focal, births, terminals, present, horizon, counter, width=1000, lag=500):
    require(type(width) is int and width > 0 and type(lag) is int and lag > 0, 'positive width/lag required')
    metrics = lineage_metrics(founders, focal, births, terminals, present, horizon, counter)
    focal = entity(focal)
    depths = {focal: 0}
    members = {}
    for event in births:
        parent, child = entity(event['parent']), entity(event['child'])
        if parent in depths:
            depths[child] = depths[parent]+1
            members[child] = (event['tick'], depths[child])
    removed = {entity(x['id']): x for x in terminals}
    hp = {entity(x['id']): x['hp'] for x in present}
    cells = defaultdict(zero)
    flows = Counter()
    for ident, (tick, depth) in members.items():
        cohort = (tick-1)//width
        cell = cells[(cohort,depth)]
        cell['births'] += 1
        if ident in removed:
            event = removed[ident]
            cell['removed'] += 1
            trigger = event.get('trigger', 'unclassified_removal')
            flows[(cohort,depth,(event['tick']-1)//width,trigger)] += 1
            end = event['tick']
        else:
            cell['positive_hp_survivors' if hp[ident]>0 else 'pending_hp0'] += 1
            end = horizon
        cell['observed_unremoved_ticks'] += end-tick
        if tick+lag <= horizon:
            cell['eligible_500'] += 1
            cell['unremoved_at_500'] += ident not in removed or removed[ident]['tick'] > tick+lag
        else:
            cell['late_births'] += 1
            cell['late_observed_removals' if ident in removed else 'late_censored_present'] += 1
    for value in cells.values():
        require(value['births']==value['removed']+value['positive_hp_survivors']+value['pending_hp0'], 'cell partition mismatch')
        require(value['late_births']==value['late_observed_removals']+value['late_censored_present'], 'late followup mismatch')
        require(value['births']==value['eligible_500']+value['late_births'], 'age followup partition mismatch')
    total = {key:sum(x[key] for x in cells.values()) for key in COUNTS}
    require(total['births']==metrics['lineage_births'], 'lineage birth total mismatch')
    require(sum(value['births'] for (cohort,depth),value in cells.items() if depth==1)==metrics['direct_births'], 'direct birth total mismatch')
    require(total['positive_hp_survivors']==metrics['living_descendants'] and
            total['pending_hp0']==metrics['present_hp_nonpositive_descendants'] and
            total['removed']==metrics['removed_descendants'], 'endpoint totals mismatch')
    require(sum(flows.values())==total['removed'], 'removal flow mismatch')
    return dict(cells=dict(cells), flows=dict(flows), metrics=metrics, total=total)


def write_csv(path, rows):
    rows = list(rows)
    require(bool(rows), f'no rows: {path}')
    with path.open('x',encoding='utf-8',newline='') as stream:
        writer=csv.DictWriter(stream,fieldnames=list(rows[0]))
        writer.writeheader();writer.writerows(rows)


def group_values(result, group):
    kind, cohort, depth = group
    return {key:sum(value[key] for (c,d),value in result['cells'].items()
                    if (kind not in ['cell','birth_cohort'] or c==cohort)
                    and (kind not in ['cell','generation'] or d==depth)) for key in COUNTS}


def analyze(study, out):
    require(study.resolve()!=out.resolve() and not out.exists(), 'use new output directory')
    run = json.loads((study/'run.json').read_text())
    quality = json.loads((study/'analysis/quality.json').read_text())
    require(run['status']=='completed' and quality['passed'] and quality['arms']==768, 'source study incomplete')
    for name,digest in quality['input_sha256'].items():
        require(sha256(study/name)==digest, f'changed source evidence: {name}')
    sig = run['signature']
    # Historical data is verified against its recorded execution version, not relabelled with this analysis HEAD.
    for name in ['tools/run_confirmation.py','tools/confirmation_metrics.py','tools/run_resistance_pilot.py','tools/run_metadata.py']:
        require(sha256(ROOT/name)==sig['files'][name], f'historical validator changed: {name}')
    out.mkdir(parents=True)
    started=time.monotonic()
    results = {}
    seeds=list(range(34501,34533))
    for index,case in enumerate(cases(seeds),1):
        saved = load_run(study/'runs',case,6000,sig)
        initial=endpoint=None
        births=[];terminals=[]
        with gzip.open(study/'runs'/f'{name_for(case)}.jsonl.gz','rt',encoding='utf-8') as stream:
            for line in stream:
                row=json.loads(line)
                if row['kind']=='initial':initial=row
                elif row['kind']=='birth':births.append(row)
                elif row['kind']=='removal':terminals.append(row)
                elif row['kind']=='snapshot' and row['tick']==6000:endpoint=row
        require(initial is not None and endpoint is not None,'missing initial/endpoint')
        result=decompose(initial['founders'],initial['focal_id'],births,terminals,
                         endpoint['present'],6000,endpoint['focal_birth_counter'])
        require(json.loads(json.dumps(result['metrics']))==saved['endpoints']['6000'], 'source metrics not reproduced')
        results[case]=result
        require(time.monotonic()-started<600,'10-minute analysis cap')
        if index%192==0:print(f'Validated/decomposed {index}/768 existing arms',flush=True)
    max_depth=max((d for result in results.values() for c,d in result['cells']),default=1)
    groups=[('all',-1,0)]+[('birth_cohort',c,0) for c in range(6)]
    groups += [('generation',-1,d) for d in range(1,max_depth+1)]
    groups += [('cell',c,d) for c in range(6) for d in range(1,max_depth+1)]
    arm_rows=[];flow_rows=[];means=[];deltas=[];summaries=[]
    for case,result in results.items():
        seed,env,ty,level,defense=case
        for (cohort,depth),value in sorted(result['cells'].items()):
            arm_rows.append(dict(seed=seed,environment=env,toxin_type=ty,level=level,defense=defense,
                cohort_start_exclusive=cohort*1000,cohort_end_inclusive=(cohort+1)*1000,depth=depth,
                **value,unremoved_500_rate=None if value['eligible_500']==0 else value['unremoved_at_500']/value['eligible_500']))
        for (cohort,depth,death_cohort,trigger),count in sorted(result['flows'].items()):
            flow_rows.append(dict(seed=seed,environment=env,toxin_type=ty,level=level,defense=defense,
                birth_cohort=cohort,depth=depth,removal_cohort=death_cohort,removal_trigger=trigger,count=count))
    for env in ['A','B','off']:
        for ty in [0,1]:
            for group in groups:
                kind,cohort,depth=group
                for level in ['low','high']:
                    for defense in ['off','on']:
                        values=[group_values(results[(s,env,ty,level,defense)],group) for s in seeds]
                        eligible=sum(x['eligible_500'] for x in values)
                        means.append(dict(environment=env,toxin_type=ty,group=kind,cohort=cohort,depth=depth,
                            level=level,defense=defense,seeds=32,
                            **{k:sum(x[k] for x in values)/32 for k in COUNTS},
                            eligible_seed_count=sum(x['eligible_500']>0 for x in values),
                            pooled_unremoved_500_rate=None if eligible==0 else sum(x['unremoved_at_500'] for x in values)/eligible))
                by_comparison=defaultdict(list)
                for seed in seeds:
                    arms={(level,defense):group_values(results[(seed,env,ty,level,defense)],group)
                          for level in ['low','high'] for defense in ['off','on']}
                    for comparison in ['on_high_minus_low','off_high_minus_low','interaction']:
                        on={k:arms[('high','on')][k]-arms[('low','on')][k] for k in COUNTS}
                        off={k:arms[('high','off')][k]-arms[('low','off')][k] for k in COUNTS}
                        value=on if comparison.startswith('on_') else off if comparison.startswith('off_') else {k:on[k]-off[k] for k in COUNTS}
                        require(value['births']==value['removed']+value['positive_hp_survivors']+value['pending_hp0'], 'contrast identity mismatch')
                        by_comparison[comparison].append(value)
                        deltas.append(dict(seed=seed,environment=env,toxin_type=ty,group=kind,cohort=cohort,depth=depth,
                                           comparison=comparison,**value))
                for comparison,values in by_comparison.items():
                    summaries.append(dict(environment=env,toxin_type=ty,group=kind,cohort=cohort,depth=depth,
                        comparison=comparison,seeds=32,**{k:sum(x[k] for x in values)/32 for k in COUNTS},
                        positive_birth_seeds=sum(x['births']>0 for x in values),
                        negative_birth_seeds=sum(x['births']<0 for x in values),zero_birth_seeds=sum(x['births']==0 for x in values)))
    # All strata sum back to the all-lineage seed contrast, including zero/missing strata.
    lookup={(x['seed'],x['environment'],x['toxin_type'],x['comparison'],x['group'],x['cohort'],x['depth']):x for x in deltas}
    for seed,env,ty,comparison in ((s,e,t,c) for s in seeds for e in ['A','B','off'] for t in [0,1]
                                   for c in ['on_high_minus_low','off_high_minus_low','interaction']):
        whole=lookup[(seed,env,ty,comparison,'all',-1,0)]
        for metric in COUNTS:
            total=sum(lookup[(seed,env,ty,comparison,'birth_cohort',c,0)][metric] for c in range(6))
            require(total==whole[metric],'birth-cohort addition mismatch')
            require(sum(lookup[(seed,env,ty,comparison,'generation',-1,d)][metric] for d in range(1,max_depth+1))==whole[metric], 'generation addition mismatch')
    write_csv(out/'arm_cells.csv',arm_rows)
    write_csv(out/'removal_flows.csv',flow_rows)
    write_csv(out/'group_means.csv',means)
    write_csv(out/'seed_contrasts.csv',deltas)
    write_csv(out/'contrast_summary.csv',summaries)
    require(sum(p.stat().st_size for p in out.iterdir())<25_000_000,'25MB output cap')
    for name,digest in quality['input_sha256'].items():
        require(sha256(study/name)==digest,'source modified during analysis')
    manifest={p.relative_to(ROOT).as_posix():sha256(p) for p in [Path(__file__),ROOT/'tools/test_lineage_cohorts.py',ROOT/'tools/confirmation_metrics.py',ROOT/'tools/run_confirmation.py']}
    report=dict(passed=True,exploratory_only=True,simulations_run=0,arms=768,seed_units=32,
        horizon=6000,cohort_width=1000,followup_age=500,max_depth=max_depth,
        source_run_sha256=sha256(study/'run.json'),source_quality_sha256=sha256(study/'analysis/quality.json'),
        historical_execution_signature=sig['manifest_sha256'],analysis_files_sha256=manifest,
        analysis_manifest_sha256=digest_json(manifest),git_commit=command(ROOT,'git','rev-parse','HEAD'),
        git_status=command(ROOT,'git','status','--porcelain','--untracked-files=all'),
        input_sha256=quality['input_sha256'],output_sha256={p.name:sha256(p) for p in sorted(out.iterdir()) if p.is_file()},
        elapsed_seconds=time.monotonic()-started,
        limits=['Post-treatment strata are descriptive, not causal mediation.',
                'Unremoved at age500 is not positive-HP survival or reproductive success.',
                'Late births remain in endpoint counts; observed late removals and right-censoring are distinct.',
                'HP-nonpositive removal trigger does not identify toxin, starvation or predation.'])
    (out/'quality.json').write_text(json.dumps(report,separators=(',',':')),encoding='utf-8')
    print(json.dumps({k:v for k,v in report.items() if k not in ['input_sha256','git_status','analysis_files_sha256','output_sha256']}))


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('study',type=Path)
    parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args()
    analyze(args.study.resolve(),args.out.resolve())
