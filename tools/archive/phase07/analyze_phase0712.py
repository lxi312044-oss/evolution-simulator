#!/usr/bin/env python3
"""Paired seed toxin sensitivity report. Cumulative totals are not causal attribution."""
import argparse,csv,json,statistics
from pathlib import Path
CONDITIONS=['A_baseline','B_high_dose','C_slow_clearance','D_low_threshold','E_zero_cost']
METRICS=['toxin_exposure_events','toxin_exposure_total','toxin_clearance_total','toxin_hp_damage_total','toxin_damage_ticks','toxin_plant_energy_cost_total','toxin_deaths_with_burden','toxin_deaths_with_burden_and_hp_zero']
POP=['plants','herbivores','carnivores']
def main():
 p=argparse.ArgumentParser();p.add_argument('--results',default='results/phase0712');p.add_argument('--ticks',type=int,default=50000);p.add_argument('--seed-start',type=int,default=12345);p.add_argument('--seed-count',type=int,default=5);p.add_argument('--conditions',nargs='+',choices=CONDITIONS,default=CONDITIONS);a=p.parse_args()
 root=Path(__file__).resolve().parent;out=root/a.results;records=[]
 for condition in a.conditions:
  for seed in range(a.seed_start,a.seed_start+a.seed_count):
   f=out/condition/f'seed-{seed}.csv'
   with f.open(newline='',encoding='utf-8-sig') as h:rows=list(csv.DictReader(h))
   if not rows or int(rows[-1]['tick'])!=a.ticks:raise ValueError(f'incomplete {f}')
   final=rows[-1];missing=[k for k in METRICS if k not in final]
   if missing:raise ValueError(f'missing columns {f}: {missing}')
   data={k:float(final[k]) for k in METRICS}
   if any(v<0 for v in data.values()):raise ValueError(f'negative diagnostic {f}')
   for k in METRICS:
    vals=[float(row[k]) for row in rows]
    if any(b+1e-7<a for a,b in zip(vals,vals[1:])):raise ValueError(f'nonmonotonic {k}: {f}')
   for k in POP:
    if k in final:data[k]=int(float(final[k]))
   records.append({'condition':condition,'seed':seed,**data})
 by={(x['condition'],x['seed']):x for x in records}
 print('condition | seeds | exposures | damage HP | damage ticks | plant cost | herbivores mean | carnivores surviving seeds')
 for condition in a.conditions:
  rr=[x for x in records if x['condition']==condition]
  def total(k):return sum(x[k] for x in rr)
  herb=f"{statistics.mean(x['herbivores'] for x in rr):.1f}" if all('herbivores' in x for x in rr) else 'N/A'
  surviving=sum(x.get('carnivores',0)>0 for x in rr)
  print(f"{condition} | {len(rr)} | {total('toxin_exposure_events'):.0f} | {total('toxin_hp_damage_total'):.9g} | {total('toxin_damage_ticks'):.0f} | {total('toxin_plant_energy_cost_total'):.3f} | {herb} | {surviving}")
 pairs=[]
 if 'A_baseline' in a.conditions:
  for condition in a.conditions:
   if condition=='A_baseline':continue
   for seed in range(a.seed_start,a.seed_start+a.seed_count):
    x=by[(condition,seed)];b=by[('A_baseline',seed)]
    pairs.append({'condition':condition,'seed':seed,'delta_hp_damage':x['toxin_hp_damage_total']-b['toxin_hp_damage_total'],'delta_plant_cost':x['toxin_plant_energy_cost_total']-b['toxin_plant_energy_cost_total'],**{f'delta_{k}':x[k]-b[k] for k in POP if k in x and k in b}})
 out.mkdir(parents=True,exist_ok=True)
 dest=out/'sensitivity_report.json';dest.write_text(json.dumps({'ticks':a.ticks,'records':records,'paired_differences':pairs,'caveat':'Cumulative diagnostics and end populations cannot identify toxin-attributable deaths; sample size five seeds.'},indent=2),encoding='utf-8');print('Saved',dest)
if __name__=='__main__':main()
