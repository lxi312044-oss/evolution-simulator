#!/usr/bin/env python3
import argparse,csv,json,math,statistics
from pathlib import Path

def main():
 p=argparse.ArgumentParser();p.add_argument('--root',default='results/phase07221/runs');p.add_argument('--out',default='results/phase07221/analysis');p.add_argument('--ticks',type=int,default=50000);p.add_argument('--interval',type=int,default=100);p.add_argument('--expected-seeds',type=int,default=20);a=p.parse_args()
 out=Path(a.out);out.mkdir(parents=True,exist_ok=True);issues=[];runs=[];trajectories=[];keys=set()
 for path in sorted(Path(a.root).glob('*.resource-reproduction.csv')):
  try:
   stem=path.name.removesuffix('.resource-reproduction.csv');prefix,level=stem.rsplit('-',1);condition,seedstr=prefix.rsplit('-seed',1);seed=int(seedstr)
   if (condition,level) not in [('B_no_toxin_cost','control'),('F_strong_cost','x1'),('F_strong_cost','x2'),('F_strong_cost','x4')]:raise ValueError('invalid condition')
   if (condition,seed,level) in keys:raise ValueError('duplicate')
   keys.add((condition,seed,level))
   with path.open(encoding='utf-8-sig',newline='') as f:reader=csv.DictReader(f);fields=set(reader.fieldnames or []);rows=list(reader)
   required={'tick','seed','herbivores','resistance_mean','herbivore_births_delta','toxin_exposure_events_delta','resistance_low_alive','resistance_mid_alive','resistance_high_alive','resistance_low_parent_births_delta','resistance_mid_parent_births_delta','resistance_high_parent_births_delta','resistance_low_damage_alive','resistance_mid_damage_alive','resistance_high_damage_alive'}
   if required-fields:raise ValueError('missing columns '+str(required-fields))
   if [int(r['tick']) for r in rows]!=list(range(0,a.ticks+1,a.interval)):raise ValueError('tick coverage')
   if any(int(r['seed'])!=seed for r in rows):raise ValueError('seed mismatch')
   herb=[int(r['herbivores']) for r in rows];first=float(rows[0]['resistance_mean']);last=float(rows[-1]['resistance_mean'])
   births=sum(int(r['herbivore_births_delta']) for r in rows);bins=['low','mid','high'];parent=[sum(int(r[f'resistance_{b}_parent_births_delta']) for r in rows) for b in bins]
   if sum(parent)!=births:issues.append(stem+': parent births mismatch')
   if any(sum(int(r[f'resistance_{b}_alive']) for b in bins)!=int(r['herbivores']) for r in rows):issues.append(stem+': alive bin mismatch')
   runs.append(dict(condition=condition,seed=seed,level=level,extinct=int(0 in herb),minimum_herbivores=min(herb),final_herbivores=herb[-1],births=births,exposures=sum(int(r['toxin_exposure_events_delta']) for r in rows),initial_resistance=first,final_resistance=last,resistance_change=last-first if math.isfinite(last) else float('nan'),low_parent_births=parent[0],mid_parent_births=parent[1],high_parent_births=parent[2],final_high_resistance_share=(int(rows[-1]['resistance_high_alive'])/herb[-1] if herb[-1] else float('nan'))))
   for r in rows:trajectories.append(dict(condition=condition,seed=seed,level=level,tick=r['tick'],herbivores=r['herbivores'],resistance_mean=r['resistance_mean'],low_alive=r['resistance_low_alive'],mid_alive=r['resistance_mid_alive'],high_alive=r['resistance_high_alive'],low_parent_births_delta=r['resistance_low_parent_births_delta'],mid_parent_births_delta=r['resistance_mid_parent_births_delta'],high_parent_births_delta=r['resistance_high_parent_births_delta'],low_damage_alive=r['resistance_low_damage_alive'],mid_damage_alive=r['resistance_mid_damage_alive'],high_damage_alive=r['resistance_high_damage_alive']))
  except (ValueError,KeyError,IndexError) as e:issues.append(f'{path.name}: {e}')
 for c,l in [('B_no_toxin_cost','control'),('F_strong_cost','x1'),('F_strong_cost','x2'),('F_strong_cost','x4')]:
  n=sum(r['condition']==c and r['level']==l for r in runs)
  if n!=a.expected_seeds:issues.append(f'{c}/{l}: expected {a.expected_seeds}, found {n}')
 def write(name,data):
  if data:
   with (out/name).open('w',encoding='utf-8',newline='') as f:w=csv.DictWriter(f,fieldnames=list(data[0]));w.writeheader();w.writerows(data)
 write('run_summary.csv',runs);write('resistance_timeseries.csv',trajectories)
 by={(r['condition'],r['seed'],r['level']):r for r in runs};pairs=[]
 for seed in sorted({r['seed'] for r in runs}):
  base=by.get(('F_strong_cost',seed,'x1'))
  for level in ['x2','x4']:
   alt=by.get(('F_strong_cost',seed,level))
   if base and alt:pairs.append(dict(seed=seed,level=level,delta_resistance_change=alt['resistance_change']-base['resistance_change'],delta_births=alt['births']-base['births'],delta_minimum_herbivores=alt['minimum_herbivores']-base['minimum_herbivores'],base_extinct=base['extinct'],strong_extinct=alt['extinct']))
   else:issues.append(f'unpaired F seed={seed} {level}')
 write('paired_comparison.csv',pairs)
 quality={'runs':len(runs),'paired_comparisons':len(pairs),'issues':issues,'limitations':['Alive-bin damage is survivor-only and cannot establish survival rates.','Parent birth counts do not measure per-individual lifetime reproductive success.','Toxin-strength effects on resistance mean are not alone proof of selection.','All strengths use the same nominal seed but different RNG paths after divergence.']}
 (out/'quality.json').write_text(json.dumps(quality,indent=2,ensure_ascii=False),encoding='utf-8')
 lines=['# Phase 07.2.21: toxin-strength selection experiment','',f'Runs: {len(runs)}, paired comparisons: {len(pairs)}, issues: {len(issues)}','','|Condition|Strength|Extinctions|Median births|Median resistance change|','|---|---|---:|---:|---:|']
 for c,l in [('B_no_toxin_cost','control'),('F_strong_cost','x1'),('F_strong_cost','x2'),('F_strong_cost','x4')]:
  subset=[r for r in runs if (r['condition'],r['level'])==(c,l)]
  if subset:lines.append(f"|{c}|{l}|{sum(r['extinct'] for r in subset)}/{len(subset)}|{statistics.median(r['births'] for r in subset):.1f}|{statistics.median(r['resistance_change'] for r in subset if math.isfinite(r['resistance_change'])) if any(math.isfinite(r['resistance_change']) for r in subset) else float('nan'):.6f}|")
 lines+=['','## Important interpretation','Resistance evolution is the primary objective. Check paired shifts and births, not only extinction.','The resistance bins and survivor-only cumulative damage are descriptive, not direct causal estimates of selection or mortality.','For a definitive fitness estimate, add per-individual birth/death event records and compare exposure-adjusted survival and lifetime offspring.']
 (out/'REPORT.md').write_text('\n'.join(lines)+'\n',encoding='utf-8')
 print(json.dumps(quality,ensure_ascii=False));return 2 if issues else 0
if __name__=='__main__':raise SystemExit(main())
