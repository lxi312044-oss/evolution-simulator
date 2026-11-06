#!/usr/bin/env python3
"""Phase 07.2.20: prioritize genetic resistance evolution, not just survival."""
import argparse,csv,json,math,pathlib,statistics
from collections import defaultdict

def median(xs):
    xs=[x for x in xs if math.isfinite(x)]
    return statistics.median(xs) if xs else float('nan')
def main():
    p=argparse.ArgumentParser()
    p.add_argument('--root',default='results/phase07220/runs');p.add_argument('--out',default='results/phase07220/analysis')
    p.add_argument('--ticks',type=int,default=50000);p.add_argument('--interval',type=int,default=100);p.add_argument('--expected-seeds',type=int,default=20)
    a=p.parse_args();out=pathlib.Path(a.out);out.mkdir(parents=True,exist_ok=True)
    modes=['off','p50','p100','p200']; conditions=['B_no_toxin_cost','F_strong_cost']; issues=[];summaries=[]; trajectories=[]; keys=set()
    required={'tick','seed','herbivores','plants','soil_n','herbivore_births_delta','spontaneous_births_delta','resistance_mean','resistance_variance','resistance_birth_mean','resistance_parent_birth_mean','resistance_mutation_attempts_delta','resistance_birth_events_delta','toxin_exposure_events_delta'}
    for path in sorted(pathlib.Path(a.root).glob('*.resource-reproduction.csv')):
        try:
            stem=path.name.removesuffix('.resource-reproduction.csv');prefix,mode=stem.rsplit('-',1);cond,seedstr=prefix.rsplit('-seed',1);seed=int(seedstr)
            if cond not in conditions or mode not in modes:raise ValueError('unknown condition/mode')
            if (cond,seed,mode) in keys:raise ValueError('duplicate run')
            keys.add((cond,seed,mode))
            with path.open(encoding='utf-8-sig',newline='') as f:reader=csv.DictReader(f);missing=required-set(reader.fieldnames or []);rows=list(reader)
            if missing:raise ValueError(f'missing columns {sorted(missing)}')
            if [int(r['tick']) for r in rows]!=list(range(0,a.ticks+1,a.interval)):raise ValueError('tick coverage')
            if any(int(r['seed'])!=seed for r in rows):raise ValueError('seed mismatch')
            herb=[int(r['herbivores']) for r in rows]; plants=[int(r['plants']) for r in rows]
            def v(r,k):return float(r[k])
            first=v(rows[0],'resistance_mean');last=v(rows[-1],'resistance_mean')
            birth_events=sum(int(r['resistance_birth_events_delta']) for r in rows)
            births=sum(int(r['herbivore_births_delta']) for r in rows)
            if birth_events!=births:issues.append(f'{stem}: genealogy births {birth_events} != {births}')
            if mode=='off' and sum(int(r['spontaneous_births_delta']) for r in rows)!=0:issues.append(f'{stem}: OFF generated plants')
            summary=dict(condition=cond,seed=seed,mode=mode,min_herbivores=min(herb),extinct=int(0 in herb),final_herbivores=herb[-1],herbivore_births=births,spontaneous_births=sum(int(r['spontaneous_births_delta']) for r in rows),initial_resistance_mean=first,final_resistance_mean=last,resistance_change=last-first if math.isfinite(first) and math.isfinite(last) else float('nan'),final_resistance_variance=v(rows[-1],'resistance_variance'),toxin_exposures=sum(int(r['toxin_exposure_events_delta']) for r in rows),mutation_attempts=sum(int(r['resistance_mutation_attempts_delta']) for r in rows))
            summaries.append(summary)
            for r in rows:
                trajectories.append(dict(condition=cond,seed=seed,mode=mode,tick=int(r['tick']),herbivores=int(r['herbivores']),plants=int(r['plants']),resistance_mean=r['resistance_mean'],resistance_variance=r['resistance_variance'],resistance_birth_mean=r['resistance_birth_mean'],resistance_parent_birth_mean=r['resistance_parent_birth_mean'],births_delta=r['herbivore_births_delta']))
        except (ValueError,KeyError) as e:issues.append(f'{path.name}: {e}')
    for cond in conditions:
        for mode in modes:
            count=sum(r['condition']==cond and r['mode']==mode for r in summaries)
            if count!=a.expected_seeds:issues.append(f'{cond}/{mode}: {count} runs, expected {a.expected_seeds}')
    bykey={(r['condition'],r['seed'],r['mode']):r for r in summaries}; paired=[]
    for cond in conditions:
        for seed in sorted({r['seed'] for r in summaries if r['condition']==cond}):
            off=bykey.get((cond,seed,'off'))
            for mode in modes[1:]:
                on=bykey.get((cond,seed,mode))
                if not off or not on:issues.append(f'unpaired {cond}/{seed}/{mode}');continue
                paired.append(dict(condition=cond,seed=seed,mode=mode,delta_resistance_change=on['resistance_change']-off['resistance_change'],delta_herbivore_births=on['herbivore_births']-off['herbivore_births'],delta_min_herbivores=on['min_herbivores']-off['min_herbivores'],off_extinct=off['extinct'],on_extinct=on['extinct']))
    def write(name,records):
        if records:
            with (out/name).open('w',encoding='utf-8',newline='') as f:
                w=csv.DictWriter(f,fieldnames=list(records[0]));w.writeheader();w.writerows(records)
    write('run_summary.csv',summaries);write('paired_comparison.csv',paired);write('resistance_timeseries.csv',trajectories)
    quality=dict(runs=len(summaries),paired_comparisons=len(paired),issues=issues,limitations=['Resistance mean change alone does not establish toxin-driven selection','Surviving population variance is not a direct measure of genealogy diversity','Extinct runs have undefined final resistance','B/F comparisons must be interpreted in light of toxin exposure and costs'])
    (out/'quality.json').write_text(json.dumps(quality,indent=2,ensure_ascii=False))
    lines=['# Phase 07.2.20 — resistance evolution first','',f'Runs: {len(summaries)}; paired comparisons: {len(paired)}; issues: {len(issues)}','', '|Condition|Germination|Extinct|Median births|Median resistance change|Median final variance|','|---|---|---:|---:|---:|---:|']
    for cond in conditions:
        for mode in modes:
            subset=[r for r in summaries if r['condition']==cond and r['mode']==mode]
            if subset:lines.append(f"|{cond}|{mode}|{sum(r['extinct'] for r in subset)}/{len(subset)}|{median([r['herbivore_births'] for r in subset]):.1f}|{median([r['resistance_change'] for r in subset]):.5f}|{median([r['final_resistance_variance'] for r in subset]):.6f}|")
    lines+=['','## Interpretation','Primary goal: detect heritable toxin-resistance change across generations under actual toxin exposure. Survival and births are enabling conditions, not the endpoint.','Do not select a germination strength solely because it maximizes herbivore counts.','Check resistance trajectories, mutation, exposure, paired differences and extinction together.']
    (out/'REPORT.md').write_text('\n'.join(lines)+'\n')
    print(json.dumps(quality,ensure_ascii=False));return 2 if issues else 0
if __name__=='__main__':raise SystemExit(main())
