#!/usr/bin/env python3
"""Phase 07.2.18: paired-seed population-cycle and recovery-window diagnostics.
No ecosystem rule changes. Descriptive timing only, not causal attribution.
"""
import argparse,csv,json,statistics,pathlib
from collections import defaultdict

def main():
 p=argparse.ArgumentParser();p.add_argument('--root',default='results/phase07218/runs');p.add_argument('--out',default='results/phase07218/analysis');p.add_argument('--start',type=int,default=2500);p.add_argument('--end',type=int,default=50000);p.add_argument('--interval',type=int,default=100);p.add_argument('--expected-seeds',type=int,default=20);p.add_argument('--plant-low-fraction',type=float,default=.25);p.add_argument('--recovery-fraction',type=float,default=.60);a=p.parse_args()
 if not (0<a.plant_low_fraction<a.recovery_fraction<1):p.error('Require 0 < plant-low-fraction < recovery-fraction < 1')
 out=pathlib.Path(a.out);out.mkdir(parents=True,exist_ok=True)
 groups=defaultdict(dict);issues=[];rows_out=[];episodes=[]
 files=sorted(pathlib.Path(a.root).glob('*.resource-reproduction.csv'))
 for path in files:
  try:condition,ss=path.name.removesuffix('.resource-reproduction.csv').rsplit('-seed',1);seed=int(ss)
  except ValueError:issues.append('bad filename '+path.name);continue
  if condition not in ('B_no_toxin_cost','F_strong_cost'):issues.append('bad condition '+condition);continue
  if seed in groups[condition]:issues.append('duplicate '+path.name);continue
  try:
   with path.open(encoding='utf-8-sig',newline='') as f:raw=list(csv.DictReader(f))
   ticks=[int(r['tick']) for r in raw]
   if ticks!=list(range(0,a.end+1,a.interval)):issues.append('tick coverage '+path.name);continue
   if any(int(r['seed'])!=seed for r in raw):issues.append('seed mismatch '+path.name);continue
   data=[r for r in raw if a.start<=int(r['tick'])<=a.end]
   if not data:issues.append('empty window '+path.name);continue
   groups[condition][seed]=data
  except (ValueError,KeyError) as e:issues.append(f'{path.name}: {e}')
 for cond,seeds in groups.items():
  for seed,data in seeds.items():
   baseline=max(float(r['plants']) for r in data)
   low=baseline*a.plant_low_fraction;high=baseline*a.recovery_fraction
   in_episode=False;start=None;trough=None;min_plants=None;min_herb=None;soil_start=None;herb_start=None
   for r in data:
    t=int(r['tick']);plants=float(r['plants']);herb=float(r['herbivores']);soil=float(r['soil_n']);births=float(r['herbivore_births_delta']);e_samples=float(r['phase24_repro_samples_delta']);energy=float(r['phase24_repro_energy_ready_delta'])
    rows_out.append({'condition':cond,'seed':seed,'tick':t,'plants':plants,'herbivores':herb,'soil_n':soil,'plant_consumed_deaths_delta':r['plant_consumed_deaths_delta'],'plant_births_delta':r['plant_births_delta'],'herbivore_births_delta':births,'herbivore_hp_deaths_delta':r['herbivore_hp_deaths_delta'],'herbivore_age_deaths_delta':r['herbivore_age_deaths_delta'],'energy_ready_rate':energy/e_samples if e_samples else '','plant_low_threshold':low,'plant_recovery_threshold':high})
    if not in_episode and plants<=low:
     in_episode=True;start=t;trough=t;min_plants=plants;min_herb=herb;soil_start=soil;herb_start=herb
    elif in_episode:
     if plants<min_plants:min_plants=plants;trough=t
     min_herb=min(min_herb,herb)
     if plants>=high:
      episodes.append({'condition':cond,'seed':seed,'start_tick':start,'trough_tick':trough,'recovery_tick':t,'duration_ticks':t-start,'min_plants':min_plants,'herbivores_at_start':herb_start,'min_herbivores_during_recovery':min_herb,'soil_n_change_to_recovery':soil-soil_start,'recovered':1})
      in_episode=False
   if in_episode:episodes.append({'condition':cond,'seed':seed,'start_tick':start,'trough_tick':trough,'recovery_tick':'','duration_ticks':int(data[-1]['tick'])-start,'min_plants':min_plants,'herbivores_at_start':herb_start,'min_herbivores_during_recovery':min_herb,'soil_n_change_to_recovery':'','recovered':0})
 def write(name,rows):
  if rows:
   with (out/name).open('w',encoding='utf-8',newline='') as f:w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
 write('same_run_cycle_100tick.csv',rows_out);write('plant_recovery_episodes.csv',episodes)
 conditions=['B_no_toxin_cost','F_strong_cost'];sets=[set(groups[c]) for c in conditions]
 for c,s in zip(conditions,sets):
  if len(s)!=a.expected_seeds:issues.append(f'{c} expected {a.expected_seeds} seeds, found {len(s)}')
 if sets[0]!=sets[1]:issues.append('unpaired seeds')
 summary=[]
 for c in conditions:
  es=[r for r in episodes if r['condition']==c];completed=[r for r in es if r['recovered']]
  summary.append({'condition':c,'runs':len(groups[c]),'episodes':len(es),'completed_recoveries':len(completed),'censored_episodes':len(es)-len(completed),'median_completed_recovery_ticks':statistics.median([r['duration_ticks'] for r in completed]) if completed else '','min_herbivores_across_episodes':min((r['min_herbivores_during_recovery'] for r in es),default='')})
 write('condition_cycle_summary.csv',summary)
 quality={'runs':sum(map(len,groups.values())),'paired_seeds':len(sets[0]&sets[1]),'episodes':len(episodes),'issues':issues,'limitations':['Recovery thresholds are fractions of each run maximum, not mechanistic thresholds','Deaths and soil nutrient changes do not establish nutrient flow attribution','Plant loss and herbivore decline can be jointly driven by other mechanisms','Incomplete recovery episodes are right-censored','No spontaneous germination or simulation logic changes']}
 (out/'quality.json').write_text(json.dumps(quality,ensure_ascii=False,indent=2),encoding='utf-8')
 (out/'REPORT.md').write_text('# Phase 07.2.18 植物・草食周期と回復遅延の観測\n\n'+f'観測ラン: {quality["runs"]}、ペアseed: {quality["paired_seeds"]}、回復エピソード: {len(episodes)}、品質問題: {len(issues)}\n\n'+ '\n'.join(f'- {r["condition"]}: 回復完了 {r["completed_recoveries"]}/{r["episodes"]}、完了例の回復時間中央値 {r["median_completed_recovery_ticks"]} tick、回復中草食最小個体数 {r["min_herbivores_across_episodes"]}' for r in summary)+'\n\n**注意:** 相関や時系列の順序は因果の証明ではありません。土壌栄養の増加が草食死体由来かどうかは資源移送の内訳が必要です。自然発芽は未実装です。\n',encoding='utf-8')
 print(json.dumps(quality,ensure_ascii=False));return 2 if issues else 0
if __name__=='__main__':raise SystemExit(main())
