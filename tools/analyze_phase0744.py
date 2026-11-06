#!/usr/bin/env python3
"""Phase 07.4.4: non-invasive demographic mechanism decomposition.

Only observational proxies: germination is mixed into births and plant energy
is not observed in Phase 07.4.3 CSV. No direct mechanistic proof is claimed.
"""
import argparse
import csv
import json
import math
from pathlib import Path

CASES = ('zero', 'standard', 'high')
SEEDS = (12345, 12346, 12347)
FIELDS = ('toxic_births','nontoxic_births','toxic_age_deaths','nontoxic_age_deaths',
          'toxic_consumed_deaths','nontoxic_consumed_deaths','toxin_plant_energy_cost_total')

def load(path):
    with path.open(encoding='utf-8-sig',newline='') as f:
        reader=csv.DictReader(f)
        required={'seed','tick','toxic_alive','nontoxic_alive','toxic_fraction','herbivores',*FIELDS}
        if not required.issubset(reader.fieldnames or []):
            raise ValueError(f'{path}: missing {sorted(required-set(reader.fieldnames or []))}')
        rows=[]
        for raw in reader:
            row={k:float(raw[k]) for k in required}
            if not all(math.isfinite(v) for v in row.values()):raise ValueError(f'{path}: nonfinite')
            if any(row[k]<0 for k in required-{'seed','tick','toxic_fraction'}):raise ValueError(f'{path}: negative count')
            total=row['toxic_alive']+row['nontoxic_alive']
            if total and abs(row['toxic_fraction']-row['toxic_alive']/total)>2e-8:raise ValueError(f'{path}: inconsistent fraction')
            rows.append(row)
    if len(rows)<2 or rows[0]['tick']!=0:raise ValueError(f'{path}: insufficient rows or nonzero start')
    for a,b in zip(rows,rows[1:]):
        if b['tick']<=a['tick']:raise ValueError(f'{path}: nonmonotonic tick')
        if any(b[k]<a[k]-1e-7 for k in FIELDS):raise ValueError(f'{path}: decreasing cumulative counter')
        if b['seed']!=a['seed']:raise ValueError(f'{path}: inconsistent seed')
    return rows

def window(a,b,samples=None):
    dt=b['tick']-a['tick']
    samples=samples or [a,b]
    def integrate(key):
        return sum((x[key]+y[key])/2*(y['tick']-x['tick']) for x,y in zip(samples,samples[1:]))
    result={'start_tick':int(a['tick']),'end_tick':int(b['tick']),
            'initial_toxic_fraction':a['toxic_fraction'],'final_toxic_fraction':b['toxic_fraction'],
            'delta_toxic_fraction':b['toxic_fraction']-a['toxic_fraction'],
            'mean_herbivores_approx':integrate('herbivores')/dt,
            'toxin_energy_cost':b['toxin_plant_energy_cost_total']-a['toxin_plant_energy_cost_total']}
    for trait,prefix in [('toxic','toxic'),('nontoxic','nontoxic')]:
        plant_ticks=integrate(f'{trait}_alive')
        births=b[f'{prefix}_births']-a[f'{prefix}_births']
        age=b[f'{prefix}_age_deaths']-a[f'{prefix}_age_deaths']
        eaten=b[f'{prefix}_consumed_deaths']-a[f'{prefix}_consumed_deaths']
        result[trait]={'start_alive':a[f'{trait}_alive'],'end_alive':b[f'{trait}_alive'],
                       'plant_ticks_approx':plant_ticks,'births':births,'age_deaths':age,
                       'consumed_deaths':eaten,'net_birth_minus_deaths':births-age-eaten,
                       'population_delta':b[f'{trait}_alive']-a[f'{trait}_alive'],
                       'accounting_residual':(b[f'{trait}_alive']-a[f'{trait}_alive'])-(births-age-eaten),
                       'birth_rate':births/plant_ticks if plant_ticks else None,
                       'age_death_rate':age/plant_ticks if plant_ticks else None,
                       'consumed_death_rate':eaten/plant_ticks if plant_ticks else None}
    for key in ('birth_rate','age_death_rate','consumed_death_rate'):
        x=result['toxic'][key];y=result['nontoxic'][key]
        result['toxic_minus_nontoxic_'+key]=x-y if x is not None and y is not None else None
    return result

def analyze(folder,seeds=SEEDS,cases=CASES,window_ticks=2500):
    runs=[]
    for seed in seeds:
        for case in cases:
            rows=load(folder/f'{case}-seed{seed}.csv')
            if any(int(r['seed'])!=seed for r in rows):raise ValueError('wrong seed')
            if rows[-1]['tick']%window_ticks:raise ValueError('window must divide final tick')
            by_tick={int(r['tick']):r for r in rows}
            endpoints=list(range(0,int(rows[-1]['tick'])+1,window_ticks))
            if any(t not in by_tick for t in endpoints):raise ValueError('missing window boundary')
            windows=[window(by_tick[a],by_tick[b],[r for r in rows if a<=r['tick']<=b]) for a,b in zip(endpoints,endpoints[1:])]
            full=window(rows[0],rows[-1],rows)
            runs.append({'seed':seed,'case':case,'full':full,'windows':windows})
    paired=[]
    for seed in seeds:
        baseline=next(r for r in runs if r['seed']==seed and r['case']=='zero')
        for case in cases:
            if case=='zero':continue
            r=next(r for r in runs if r['seed']==seed and r['case']==case)
            delta={}
            for key in ('delta_toxic_fraction','mean_herbivores_approx'):
                delta[key]=r['full'][key]-baseline['full'][key]
            for trait in ('toxic','nontoxic'):
                for metric in ('birth_rate','age_death_rate','consumed_death_rate'):
                    v=r['full'][trait][metric];v0=baseline['full'][trait][metric]
                    delta[f'{trait}_{metric}']=v-v0 if v is not None and v0 is not None else None
            paired.append({'seed':seed,'case':case,'vs':'zero','delta':delta})
    flags=[]
    for r in runs:
        for w in r['windows']:
            for trait in ('toxic','nontoxic'):
                if abs(w[trait]['accounting_residual'])>1e-6:
                    flags.append({'seed':r['seed'],'case':r['case'],'window_start':w['start_tick'],
                                  'trait':trait,'residual':w[trait]['accounting_residual']})
    return {'phase':'07.4.4','status':'PASS' if not flags else 'REVIEW',
            'source':'Phase 07.4.3 nine CSVs; no additional simulation runs',
            'window_ticks':window_ticks,'runs':runs,'paired_vs_zero':paired,
            'accounting_flags':flags,
            'limitations':['Birth counts include spontaneous germination and mutated offspring; cannot infer individual reproductive fitness.',
                           'Plant energy/reserve and soil nutrients are not recorded in the source CSV: direct energy mediation cannot be tested.',
                           'Per-plant-tick denominators use trapezoidal interpolation between every 100-tick observation; subinterval changes are unobserved.',
                           'Herbivore population is a proxy for grazing pressure, not observed encounter or consumption intensity.',
                           'Changes in cost alter subsequent trajectories and resource circulation; comparisons do not isolate a direct pathway.',
                           'Three seeds are exploratory; no statistical significance or toxin-defense benefit inferred.']}

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--input',type=Path,default=Path('results/phase0743'))
    p.add_argument('--output',type=Path,default=Path('results/phase0744'))
    p.add_argument('--window',type=int,default=2500)
    a=p.parse_args()
    if a.window<=0:raise ValueError('window must be positive')
    report=analyze(a.input,window_ticks=a.window)
    a.output.mkdir(parents=True,exist_ok=True)
    (a.output/'REPORT.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    with (a.output/'WINDOWS.csv').open('w',newline='',encoding='utf-8') as f:
        cols=['seed','case','start_tick','end_tick','delta_toxic_fraction','mean_herbivores_approx','toxin_energy_cost',
              'toxic_birth_rate','nontoxic_birth_rate','toxic_consumed_death_rate','nontoxic_consumed_death_rate',
              'toxic_age_death_rate','nontoxic_age_death_rate','toxic_accounting_residual','nontoxic_accounting_residual']
        writer=csv.DictWriter(f,fieldnames=cols);writer.writeheader()
        for run in report['runs']:
            for w in run['windows']:
                row={'seed':run['seed'],'case':run['case'],**{k:w[k] for k in ('start_tick','end_tick','delta_toxic_fraction','mean_herbivores_approx','toxin_energy_cost')}}
                for t in ('toxic','nontoxic'):
                    for k in ('birth_rate','consumed_death_rate','age_death_rate','accounting_residual'):
                        row[f'{t}_{k}']=w[t][k]
                writer.writerow(row)
    print(f"{report['status']} Phase 07.4.4 demographic decomposition; accounting flags={len(report['accounting_flags'])}")
    print('Report:',a.output/'REPORT.json')
    if report['accounting_flags']:raise SystemExit(2)
if __name__=='__main__':main()
