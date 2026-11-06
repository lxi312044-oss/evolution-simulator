#!/usr/bin/env python3
"""Paired OFF/ON diagnostics. Descriptive comparison, not causal proof across seeds."""
import argparse,csv,json,pathlib,statistics
from collections import defaultdict

def main():
 p=argparse.ArgumentParser();p.add_argument('--root',default='results/phase07219/runs');p.add_argument('--out',default='results/phase07219/analysis');p.add_argument('--ticks',type=int,default=50000);p.add_argument('--interval',type=int,default=100);p.add_argument('--expected-seeds',type=int,default=20);a=p.parse_args()
 out=pathlib.Path(a.out);out.mkdir(parents=True,exist_ok=True)
 groups=defaultdict(dict);issues=[];summary=[]
 for path in sorted(pathlib.Path(a.root).glob('*.resource-reproduction.csv')):
  try:
   stem=path.name.removesuffix('.resource-reproduction.csv');before,mode=stem.rsplit('-',1);condition,seed=before.rsplit('-seed',1);seed=int(seed)
   if condition not in ('B_no_toxin_cost','F_strong_cost') or mode not in ('on','off'):raise ValueError('unexpected condition/mode')
   with path.open(encoding='utf-8-sig',newline='') as f:rows=list(csv.DictReader(f))
   if [int(r['tick']) for r in rows]!=list(range(0,a.ticks+1,a.interval)):raise ValueError('tick coverage')
   if any(int(r['seed'])!=seed for r in rows):raise ValueError('seed mismatch')
   if (seed,mode) in groups[condition]:raise ValueError('duplicate')
   groups[condition][seed,mode]=rows
   plants=[int(r['plants']) for r in rows];herbs=[int(r['herbivores']) for r in rows]
   summary.append(dict(condition=condition,seed=seed,mode=mode,min_herbivores=min(herbs),final_herbivores=herbs[-1],herbivore_extinct=int(0 in herbs),min_plants=min(plants),final_plants=plants[-1],plant_births=sum(int(r['plant_births_delta']) for r in rows),spontaneous_births=sum(int(r['spontaneous_births_delta']) for r in rows),herbivore_births=sum(int(r['herbivore_births_delta']) for r in rows)))
  except (ValueError,KeyError) as e:issues.append(f'{path.name}: {e}')
 paired=[]
 bykey={(r['condition'],r['seed'],r['mode']):r for r in summary}
 for cond in ('B_no_toxin_cost','F_strong_cost'):
  seeds={seed for seed,mode in groups[cond]}
  if len(seeds)!=a.expected_seeds:issues.append(f'{cond}: expected {a.expected_seeds} seeds, got {len(seeds)}')
  for seed in sorted(seeds):
   if (cond,seed,'on') not in bykey or (cond,seed,'off') not in bykey:issues.append(f'unpaired {cond} seed {seed}');continue
   off=bykey[cond,seed,'off'];on=bykey[cond,seed,'on']
   paired.append(dict(condition=cond,seed=seed,delta_min_herbivores=on['min_herbivores']-off['min_herbivores'],delta_final_herbivores=on['final_herbivores']-off['final_herbivores'],delta_herbivore_births=on['herbivore_births']-off['herbivore_births'],off_extinct=off['herbivore_extinct'],on_extinct=on['herbivore_extinct'],spontaneous_births=on['spontaneous_births']))
  for seed in seeds:
   if (cond,seed,'off') in bykey and bykey[cond,seed,'off']['spontaneous_births']!=0:issues.append(f'OFF spontaneous births {cond} {seed}')
 def write(name,rows):
  if rows:
   with (out/name).open('w',newline='',encoding='utf-8') as f:w=csv.DictWriter(f,fieldnames=rows[0].keys());w.writeheader();w.writerows(rows)
 write('run_summary.csv',summary);write('paired_comparison.csv',paired)
 quality=dict(runs=len(summary),pairs=len(paired),issues=issues,limitations=['A higher plant count alone does not prove improved long-term evolution','Minimum across all ticks includes initial conditions','Separate RNG stream keeps baseline OFF reproducible; ON changes future ecosystem interactions','Initial germination energy is accounted for as sunlight input'])
 (out/'quality.json').write_text(json.dumps(quality,ensure_ascii=False,indent=2),encoding='utf-8')
 lines=['# Phase 07.2.19 ON/OFF comparison','',f'Runs: {len(summary)}; paired: {len(paired)}; issues: {len(issues)}','']
 for cond in ('B_no_toxin_cost','F_strong_cost'):
  ps=[r for r in paired if r['condition']==cond]
  if ps:lines.append(f'- {cond}: median Δ final herbivores {statistics.median(r["delta_final_herbivores"] for r in ps)}, median Δ herbivore births {statistics.median(r["delta_herbivore_births"] for r in ps)}, ON spontaneous births total {sum(r["spontaneous_births"] for r in ps)}')
 lines.extend(['','## Caveats','- This is a paired exploratory experiment, not a guarantee of population stability.','- Compare extinction frequency, trough depth, recovery and evolutionary traits before adopting as default.'])
 (out/'REPORT.md').write_text('\n'.join(lines)+'\n',encoding='utf-8')
 print(json.dumps(quality,ensure_ascii=False));return 2 if issues else 0
if __name__=='__main__':raise SystemExit(main())
