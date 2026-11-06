#!/usr/bin/env python3
"""Summarize compact Phase 07.2.15 diagnostics; no causal identification."""
import argparse,csv,json,collections,statistics
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('--root',type=Path,default=Path('results/phase07215/runs'))
p.add_argument('--out',type=Path,default=Path('results/phase07215/analysis'))
p.add_argument('--expected-ticks',type=int,default=50000)
p.add_argument('--start-seed',type=int,default=12345)
p.add_argument('--seed-count',type=int,default=20)
a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
issues=[];summaries=[];windows=[]
for condition in ('B_no_toxin_cost','F_strong_cost'):
 for seed in range(a.start_seed,a.start_seed+a.seed_count):
  path=a.root/f'{condition}-seed{seed}.reproduction.csv'
  if not path.exists(): issues.append(f'missing: {path.name}');continue
  with path.open(newline='',encoding='utf-8-sig') as f: rows=list(csv.DictReader(f))
  if not rows: issues.append(f'empty: {path.name}');continue
  try:
   ticks=[int(r['tick']) for r in rows]
   if ticks[0]!=0 or ticks[-1]!=a.expected_ticks or any(x>=y for x,y in zip(ticks,ticks[1:])): issues.append(f'invalid tick series: {path.name}')
   if any(int(r['seed'])!=seed for r in rows): issues.append(f'wrong seed: {path.name}')
   for r in rows:
    if sum(int(r[k]) for k in ('age_lt_min','age_min_to_2min','age_ge_2min'))!=int(r['herbivores']):issues.append(f'age bins mismatch: {path.name} tick={r["tick"]}')
   for start,end in ((0,3000),(3000,6000),(6000,10000),(10000,50000)):
    subset=[r for r in rows if start<int(r['tick'])<=end]
    if not subset: continue
    entry={'condition':condition,'seed':seed,'start_tick':start,'end_tick':end,'samples':len(subset),'births':sum(int(r['herbivore_births_delta']) for r in subset),'attempts':sum(int(r['herbivore_reproduction_attempts_delta']) for r in subset),'age_deaths':sum(int(r['herbivore_age_deaths_delta']) for r in subset),'hp_deaths':sum(int(r['herbivore_hp_deaths_delta']) for r in subset),'gate_samples':sum(int(r['phase24_repro_samples_delta']) for r in subset),'gate_all_ready':sum(int(r['phase24_repro_ready_all_delta']) for r in subset),'mean_population':round(statistics.mean(int(r['herbivores']) for r in subset),3)}
    for key in ('age','cooldown','hp','energy','satiety','body_n','reserve_n','transfer_e'):
     entry[f'{key}_ready']=sum(int(r[f'phase24_repro_{key}_ready_delta']) for r in subset)
    windows.append(entry)
   summaries.append({'condition':condition,'seed':seed,'final_tick':ticks[-1],'final_herbivores':int(rows[-1]['herbivores']),'total_births':sum(int(r['herbivore_births_delta']) for r in rows),'total_age_deaths':sum(int(r['herbivore_age_deaths_delta']) for r in rows),'total_hp_deaths':sum(int(r['herbivore_hp_deaths_delta']) for r in rows)})
  except (KeyError,ValueError) as e:issues.append(f'malformed {path.name}: {e}')
for name,data in [('run_summary.csv',summaries),('time_windows.csv',windows)]:
 if data:
  with (a.out/name).open('w',newline='',encoding='utf-8') as f:
   w=csv.DictWriter(f,fieldnames=list(data[0]));w.writeheader();w.writerows(data)
quality={'expected_ticks':a.expected_ticks,'expected_runs':2*a.seed_count,'condition_runs':len(summaries),'paired_seeds':len({r['seed'] for r in summaries if sum(x['seed']==r['seed'] for x in summaries)==2}),'issues':issues}
(a.out/'quality.json').write_text(json.dumps(quality,ensure_ascii=False,indent=2),encoding='utf-8')
report=['# Phase 07.2.15 observation summary','','Only locally generated compact CSVs are inspected. No original Phase 07.2.12/13 raw files are claimed to be validated.','','## Quality',f'- Runs: {len(summaries)} / {2*a.seed_count}',f'- Issues: {len(issues)}','','## Interpretation','- Gate counters are per-animal per-tick diagnostics, not counts of unique animals.','- Independent readiness counters overlap: do not add their failure counts.','- Phase 24 ready_all is not identical to actual successful births; other probabilistic and resource gates may apply.','- Age and HP death categories are the categories implemented by the simulator, not verified causal death attribution.','- Observed associations do not establish causal effects.']
for condition in ('B_no_toxin_cost','F_strong_cost'):
 sub=[r for r in windows if r['condition']==condition and r['start_tick']==3000]
 if sub:
  report+=['',f'## {condition}: 3000–6000 Tick',f'- Mean births: {statistics.mean(r["births"] for r in sub):.2f}',f'- Mean age deaths: {statistics.mean(r["age_deaths"] for r in sub):.2f}',f'- Mean HP deaths: {statistics.mean(r["hp_deaths"] for r in sub):.2f}',f'- Mean all-gates-ready observations: {statistics.mean(r["gate_all_ready"] for r in sub):.2f}']
(a.out/'REPORT.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
print(f'Runs={len(summaries)} pairs={quality["paired_seeds"]} issues={len(issues)}')
raise SystemExit(2 if issues else 0)
