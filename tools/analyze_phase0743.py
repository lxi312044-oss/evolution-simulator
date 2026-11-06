#!/usr/bin/env python3
"""Phase 07.4.3: paired, descriptive selection analysis (Python stdlib only)."""
import argparse
import csv
import json
import math
from pathlib import Path

CASES = ('zero', 'standard', 'high')
CUMULATIVE = ('toxic_births', 'nontoxic_births', 'toxic_age_deaths', 'nontoxic_age_deaths',
              'toxic_consumed_deaths', 'nontoxic_consumed_deaths', 'toxin_plant_energy_cost_total')

def load_csv(path):
    with path.open(encoding='utf-8-sig', newline='') as f:
        reader = csv.DictReader(f)
        required = {'seed','tick','toxic_alive','nontoxic_alive','toxic_fraction','herbivores', *CUMULATIVE}
        missing = required - set(reader.fieldnames or ())
        if missing:
            raise ValueError(f'{path}: missing columns {sorted(missing)}')
        rows = list(reader)
    if len(rows) < 2:
        raise ValueError(f'{path}: fewer than two observations')
    for r in rows:
        for key in required:
            r[key] = float(r[key])
            if not math.isfinite(r[key]):
                raise ValueError(f'{path}: nonfinite {key} at tick {r["tick"]}')
        total = r['toxic_alive'] + r['nontoxic_alive']
        if total < 0 or not 0 <= r['toxic_fraction'] <= 1 or (total > 0 and abs(r['toxic_fraction'] - r['toxic_alive']/total) > 2e-8):
            raise ValueError(f'{path}: invalid plant fraction at tick {r["tick"]}')
    if rows[0]['tick'] != 0:
        raise ValueError(f'{path}: expected tick 0')
    for a,b in zip(rows, rows[1:]):
        if b['tick'] <= a['tick']:
            raise ValueError(f'{path}: ticks not strictly increasing')
        for k in CUMULATIVE:
            if b[k] + 1e-7 < a[k]:
                raise ValueError(f'{path}: cumulative {k} decreased')
    return rows

def summarize(rows):
    first, last = rows[0], rows[-1]
    toxic_time = sum((a['toxic_alive']+b['toxic_alive'])/2*(b['tick']-a['tick']) for a,b in zip(rows,rows[1:]))
    non_time = sum((a['nontoxic_alive']+b['nontoxic_alive'])/2*(b['tick']-a['tick']) for a,b in zip(rows,rows[1:]))
    def delta(k): return last[k]-first[k]
    def rate(k,denom): return delta(k)/denom if denom>0 else None
    return dict(seed=int(last['seed']), ticks=int(last['tick']), initial_toxic_fraction=first['toxic_fraction'],
                final_toxic_fraction=last['toxic_fraction'], toxic_fraction_change=last['toxic_fraction']-first['toxic_fraction'],
                final_toxic_alive=int(last['toxic_alive']), final_nontoxic_alive=int(last['nontoxic_alive']),
                toxic_plant_ticks=toxic_time,nontoxic_plant_ticks=non_time,
                toxic_births=delta('toxic_births'), nontoxic_births=delta('nontoxic_births'),
                toxic_birth_rate_per_plant_tick=rate('toxic_births',toxic_time),
                nontoxic_birth_rate_per_plant_tick=rate('nontoxic_births',non_time),
                toxic_age_death_rate_per_plant_tick=rate('toxic_age_deaths',toxic_time),
                nontoxic_age_death_rate_per_plant_tick=rate('nontoxic_age_deaths',non_time),
                toxic_consumed_death_rate_per_plant_tick=rate('toxic_consumed_deaths',toxic_time),
                nontoxic_consumed_death_rate_per_plant_tick=rate('nontoxic_consumed_deaths',non_time),
                toxin_energy_cost_total=delta('toxin_plant_energy_cost_total'))

def analyze(folder, seeds, ticks):
    runs=[]
    for seed in seeds:
        for case in CASES:
            path=folder/f'{case}-seed{seed}.csv'
            rows=load_csv(path)
            if int(rows[-1]['tick']) != ticks or any(int(r['seed']) != seed for r in rows):
                raise ValueError(f'{path}: seed or final tick mismatch')
            runs.append({'case':case, **summarize(rows)})
    comparisons=[]
    for seed in seeds:
        subset={r['case']:r for r in runs if r['seed']==seed}
        baseline=subset['zero']
        for case in ('standard','high'):
            trial=subset[case]
            comparisons.append(dict(seed=seed,case=case, reference='zero',
                difference_final_toxic_fraction=trial['final_toxic_fraction']-baseline['final_toxic_fraction'],
                difference_toxic_fraction_change=trial['toxic_fraction_change']-baseline['toxic_fraction_change'],
                difference_toxic_birth_rate=(trial['toxic_birth_rate_per_plant_tick']-baseline['toxic_birth_rate_per_plant_tick']) if trial['toxic_birth_rate_per_plant_tick'] is not None and baseline['toxic_birth_rate_per_plant_tick'] is not None else None,
                difference_toxic_consumed_death_rate=(trial['toxic_consumed_death_rate_per_plant_tick']-baseline['toxic_consumed_death_rate_per_plant_tick']) if trial['toxic_consumed_death_rate_per_plant_tick'] is not None and baseline['toxic_consumed_death_rate_per_plant_tick'] is not None else None))
    aggregates={}
    for case in ('standard','high'):
        vals=[c['difference_final_toxic_fraction'] for c in comparisons if c['case']==case]
        aggregates[case]={'paired_seeds':len(vals),'mean_difference_final_toxic_fraction':sum(vals)/len(vals),
                          'min_difference':min(vals),'max_difference':max(vals),
                          'positive_seed_count':sum(v>0 for v in vals),'negative_seed_count':sum(v<0 for v in vals)}
    return dict(phase='07.4.3', status='PASS', seeds=seeds, ticks=ticks,
                cases={'zero':0.0,'standard':0.002,'high':0.02}, runs=runs,
                paired_comparisons=comparisons, aggregates=aggregates,
                limitations=['Plant-time uses trapezoidal interpolation between sampled counts.',
                             'Paired cost differences describe a treatment effect in this simulator, not a proof of the causal biological mechanism.',
                             'Births include spontaneous germination; per-plant-tick birth rate is not lineage fitness.',
                             'Predator toxin-color learning/avoidance is not implemented.',
                             'Three seeds are a pilot, not statistical evidence of general selection.'])

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--results',type=Path,default=Path('results/phase0743'))
    p.add_argument('--seeds',type=int,nargs='+',default=[12345,12346,12347])
    p.add_argument('--ticks',type=int,default=10000)
    a=p.parse_args()
    report=analyze(a.results,a.seeds,a.ticks)
    dest=a.results/'REPORT.json'
    dest.write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
    print('PASS Phase 07.4.3 paired cost analysis:',dest)
    for case,data in report['aggregates'].items():
        print(f'{case}: mean delta toxic fraction vs zero = {data["mean_difference_final_toxic_fraction"]:+.6f}; positive={data["positive_seed_count"]}; negative={data["negative_seed_count"]}')
if __name__=='__main__':main()
