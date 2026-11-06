#!/usr/bin/env python3
"""Same-run Phase 07.2.17 observation analysis; descriptive, not causal."""
import argparse,csv,json,pathlib,statistics,sys
from collections import defaultdict

def main():
 p=argparse.ArgumentParser()
 p.add_argument('--root',default='results/phase07217/runs')
 p.add_argument('--out',default='results/phase07217/analysis')
 p.add_argument('--expected-ticks',type=int,default=50000)
 p.add_argument('--interval',type=int,default=100)
 p.add_argument('--start',type=int,default=2500)
 p.add_argument('--end',type=int,default=6500)
 p.add_argument('--expected-seeds',type=int,default=20)
 a=p.parse_args();out=pathlib.Path(a.out);out.mkdir(parents=True,exist_ok=True)
 issues=[];records=[];seen=set();files=sorted(pathlib.Path(a.root).glob('*.resource-reproduction.csv'))
 conditions=('B_no_toxin_cost','F_strong_cost')
 for f in files:
  stem=f.name.removesuffix('.resource-reproduction.csv')
  try:condition,seed_str=stem.rsplit('-seed',1);seed=int(seed_str)
  except ValueError:issues.append('unexpected filename '+f.name);continue
  if condition not in conditions:issues.append('unexpected condition '+condition);continue
  if (condition,seed) in seen:issues.append('duplicate '+stem);continue
  seen.add((condition,seed))
  with f.open(encoding='utf-8-sig',newline='') as stream:rows=list(csv.DictReader(stream))
  if not rows:issues.append('empty '+stem);continue
  try:
   ticks=[int(r['tick']) for r in rows]
   if ticks!=list(range(0,a.expected_ticks+1,a.interval)):issues.append('invalid tick coverage '+stem)
   if any(int(r['seed'])!=seed for r in rows):issues.append('seed mismatch '+stem)
   for r in rows:
    t=int(r['tick'])
    if not a.start<t<=a.end:continue
    samples=float(r['phase24_repro_samples_delta'])
    d={'condition':condition,'seed':seed,'tick':t}
    for key in ('plants','soil_n','detritus_n','plant_body_n','plant_reserve_n','plant_reserve_e','plant_births_delta','plant_age_deaths_delta','plant_consumed_deaths_delta','seed_attempts_delta','seed_resource_blocked_delta','seed_capacity_blocked_delta','plant_uptake_n_delta','plant_consumed_n_delta','herbivores','herbivore_births_delta','phase24_repro_samples_delta','phase24_repro_energy_ready_delta','phase24_repro_satiety_ready_delta','local_plant_count_mean','local_plant_n_mean','local_plant_e_mean'):
     d[key]=float(r[key])
    d['energy_ready_rate']=d['phase24_repro_energy_ready_delta']/samples if samples else ''
    d['satiety_ready_rate']=d['phase24_repro_satiety_ready_delta']/samples if samples else ''
    records.append(d)
  except (ValueError,KeyError) as exc:issues.append(f'invalid data {stem}: {exc}')
 for condition in conditions:
  seeds={s for c,s in seen if c==condition}
  if len(seeds)!=a.expected_seeds:issues.append(f'{condition}: expected {a.expected_seeds} seeds, got {len(seeds)}')
 if {s for c,s in seen if c==conditions[0]}!={s for c,s in seen if c==conditions[1]}:issues.append('unpaired seeds')
 def write(name,items):
  if not items:return
  with (out/name).open('w',encoding='utf-8',newline='') as stream:
   w=csv.DictWriter(stream,fieldnames=list(items[0]));w.writeheader();w.writerows(items)
 write('same_run_100tick.csv',records)
 groups=defaultdict(list)
 for r in records:groups[(r['condition'],r['tick'])].append(r)
 summary=[]
 for (condition,tick),rows in sorted(groups.items()):
  d={'condition':condition,'tick':tick,'seeds':len(rows)}
  for key in ('plants','soil_n','detritus_n','plant_body_n','plant_reserve_e','plant_births_delta','plant_consumed_deaths_delta','herbivores','herbivore_births_delta','energy_ready_rate','satiety_ready_rate','local_plant_count_mean'):
   values=[r[key] for r in rows if isinstance(r[key],(int,float)) and r[key]==r[key]]
   d['mean_'+key]=round(statistics.mean(values),8) if values else ''
  summary.append(d)
 write('condition_time_means.csv',summary)
 bykey={(r['condition'],r['seed'],r['tick']):r for r in records};lag=[]
 for r in records:
  nxt=bykey.get((r['condition'],r['seed'],r['tick']+a.interval))
  if nxt:lag.append({'condition':r['condition'],'seed':r['seed'],'resource_tick':r['tick'],'birth_interval_end_tick':nxt['tick'],'plants':r['plants'],'soil_n':r['soil_n'],'plant_body_n':r['plant_body_n'],'local_plant_n_mean':r['local_plant_n_mean'],'energy_ready_rate':r['energy_ready_rate'],'next_herbivore_births':nxt['herbivore_births_delta'],'next_energy_ready_rate':nxt['energy_ready_rate']})
 write('lag_100tick.csv',lag)
 quality={'runs':len(seen),'paired_seeds':len({s for c,s in seen if c==conditions[0]}&{s for c,s in seen if c==conditions[1]}),'rows':len(records),'lag_rows':len(lag),'issues':issues,'limitations':['Same-run observation is not causal proof','Diagnostic ready rates use samples, not independent animals','NaN local plant means indicate zero diagnostic samples','Soil and plant stock snapshots are interval endpoints; births/deaths are interval deltas']}
 (out/'quality.json').write_text(json.dumps(quality,ensure_ascii=False,indent=2),encoding='utf-8')
 (out/'REPORT.md').write_text(f"# Phase 07.2.17 同一ラン資源・繁殖診断\n\n- Runs: {len(seen)}; paired seeds: {quality['paired_seeds']}\n- Rows: {len(records)}; lag rows: {len(lag)}\n- Quality issues: {len(issues)}\n\n同一ランの土壌・植物資源と繁殖診断を同時に観測。自然発芽は未導入。時系列相関は因果関係の証明ではない。\n".replace('\\n','\n'),encoding='utf-8')
 print(json.dumps(quality,ensure_ascii=False));return 2 if issues else 0
if __name__=='__main__':sys.exit(main())
