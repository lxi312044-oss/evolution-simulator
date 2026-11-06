#!/usr/bin/env python3
"""Join compact Phase 07.2.12 observations to the existing paired seed results."""
import argparse,csv,json,math,statistics
from pathlib import Path

def read_one(path):
    with path.open(encoding='utf-8-sig',newline='') as f:
        return list(csv.DictReader(f))

def write_csv(path, rows):
    if not rows:return
    with path.open('w',encoding='utf-8',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--root',default='results/phase07212/runs')
    ap.add_argument('--paired',default='results/phase07210/analysis/design/paired_seed_summary.csv')
    ap.add_argument('--out',default='results/phase07212/analysis')
    ap.add_argument('--expected-ticks',type=int,default=50000,help='Require every condition run to reach this tick (default: 50000)')
    args=ap.parse_args();root=Path(args.root);out=Path(args.out);out.mkdir(parents=True,exist_ok=True)
    if args.expected_ticks < 1: ap.error('--expected-ticks must be positive')
    pairs=read_one(Path(args.paired));records={};issues=[]
    for condition in ('B_no_toxin_cost','F_strong_cost'):
        for path in sorted(root.glob(f'{condition}-seed*.population-pressure.csv')):
            prefix=str(path).removesuffix('.population-pressure.csv')
            rows=read_one(path);genes=read_one(Path(prefix+'.initial-genetics.csv'))
            if not rows or len(genes)!=1:issues.append(f'Empty or missing: {prefix}');continue
            seed=int(rows[0]['seed']); ticks=[int(r['tick']) for r in rows]
            if ticks!=sorted(set(ticks)) or ticks[0]!=0:issues.append(f'Invalid tick ordering: {prefix}');continue
            if ticks[-1]!=args.expected_ticks:
                issues.append(f'Wrong duration: seed={seed} condition={condition} last_tick={ticks[-1]} expected={args.expected_ticks}')
                continue
            final=rows[-1]; population=[int(r['herbivores']) for r in rows]
            attempts=int(final['hunt_attempts_cumulative']); positive=int(final['hunt_positive_energy_cumulative']);
            r={'seed':seed,'condition':condition,'last_tick':ticks[-1],
               'founder_count':int(genes[0]['founder_count']),
               'founder_resistance_mean':float(genes[0]['mean_resistance']),
               'founder_resistance_variance':float(genes[0]['variance_resistance']),
               'mean_herbivores_sampled':statistics.mean(population),
               'min_herbivores_sampled':min(population),'max_herbivores_sampled':max(population),
               'herbivore_births':int(final['herbivore_births_cumulative']),
               'herbivore_deaths':int(final['herbivore_deaths_cumulative']),
               'hunt_attempts':attempts,'positive_energy_attacks':positive,
               'positive_attack_fraction':positive/attempts if attempts else None,
               'prey_hp_zero_after_attack':int(final['prey_hp_zero_after_attack_cumulative']),
               'toxin_exposure_events':int(final['toxin_exposure_events_cumulative']),
               'toxin_exposure_total':float(final['toxin_exposure_total_cumulative']),
               'extinct_at_sample':int(any(int(x['herbivores'])==0 for x in rows))}
            if (seed,condition) in records:issues.append(f'Duplicate: {seed} {condition}')
            records[seed,condition]=r
    expected_seeds={int(x['seed']) for x in pairs}
    for seed in sorted(expected_seeds):
        for condition in ('B_no_toxin_cost','F_strong_cost'):
            if (seed,condition) not in records: issues.append(f'Missing valid run: seed={seed} condition={condition}')
    write_csv(out/'seed_condition_covariates.csv',list(records.values()))
    joined=[]
    for pair in pairs:
        seed=int(pair['seed']);b=records.get((seed,'B_no_toxin_cost'));f=records.get((seed,'F_strong_cost'))
        if not b or not f:issues.append(f'Missing condition for paired seed {seed}');continue
        if b['last_tick']!=f['last_tick']:issues.append(f'Unequal duration: {seed}');continue
        joined.append({'seed':seed,'F_minus_B_survival':float(pair['F_minus_B_survival']),
            'F_minus_B_reproduction':float(pair['F_minus_B_reproduction']),
            'B_mean_population':b['mean_herbivores_sampled'],'F_mean_population':f['mean_herbivores_sampled'],
            'F_minus_B_mean_population':f['mean_herbivores_sampled']-b['mean_herbivores_sampled'],
            'B_hunt_attempts':b['hunt_attempts'],'F_hunt_attempts':f['hunt_attempts'],
            'F_minus_B_hunt_attempts':f['hunt_attempts']-b['hunt_attempts'],
            'B_positive_attack_fraction':b['positive_attack_fraction'],
            'F_positive_attack_fraction':f['positive_attack_fraction'],
            'F_toxin_exposure_events':f['toxin_exposure_events'],
            'F_toxin_exposure_total':f['toxin_exposure_total'],
            'B_founder_variance':b['founder_resistance_variance'],
            'F_founder_variance':f['founder_resistance_variance'],
            'B_extinct_at_sample':b['extinct_at_sample'],'F_extinct_at_sample':f['extinct_at_sample']})
    write_csv(out/'paired_covariates.csv',joined)
    covs=['F_minus_B_mean_population','B_hunt_attempts','F_hunt_attempts','F_minus_B_hunt_attempts','F_toxin_exposure_events','F_toxin_exposure_total','B_founder_variance','F_founder_variance']
    correlations=[]
    for key in covs:
        vals=[(r[key],r['F_minus_B_survival']) for r in joined if r[key] is not None and math.isfinite(r[key])]
        xs=[x for x,y in vals];ys=[y for x,y in vals]
        corr=None
        if len(vals)>=3 and len(set(xs))>1 and len(set(ys))>1: corr=statistics.correlation(xs,ys)
        correlations.append({'covariate':key,'paired_seeds':len(vals),'pearson_r_exploratory':corr,'note':'association only; cumulative counters can be affected by survival'})
    write_csv(out/'exploratory_correlations.csv',correlations)
    summary={'expected_ticks':args.expected_ticks,'paired_seeds_expected':len(pairs),'paired_seeds_joined':len(joined),'condition_runs':len(records),'issues':issues,'important_limitations':['Founder resistance may be constant by construction; do not infer seed-dependent genetic variance','Hunt attempts are not equivalent to predator-caused deaths','Cumulative exposures and attacks depend on time alive and population size','Correlations are descriptive, not causal','New runs must use identical configuration and code as prior outcome runs for strict comparability']}
    (out/'quality.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2),encoding='utf-8')
    (out/'REPORT.md').write_text('# Phase 07.2.12 — compact covariate audit\n\n'+f"- Matched pairs: {len(joined)}/{len(pairs)}\n- Condition runs: {len(records)}\n- Issues: {len(issues)}\n- Required final tick: {args.expected_ticks}\n\n"+'Observational diagnostics only; no causal claims. Existing F−B outcomes are joined by seed, not recomputed. See quality.json.\n',encoding='utf-8')
    print(f'Paired {len(joined)}/{len(pairs)}, issues={len(issues)}; output: {out}')
    if issues:raise SystemExit('Incomplete or inconsistent input; see quality.json')
if __name__=='__main__':main()
