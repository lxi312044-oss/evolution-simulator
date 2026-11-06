#!/usr/bin/env python3
"""Phase 07.2.13: temporal diagnosis from Phase 07.2.12 compact samples.
Descriptive only; exposure/hunting counters are not predator deaths or causal effects.
"""
import argparse,csv,json,math,statistics
from pathlib import Path
CONDITIONS=('B_no_toxin_cost','F_strong_cost')
COUNTERS=('herbivore_births_cumulative','herbivore_deaths_cumulative','hunt_attempts_cumulative','hunt_positive_energy_cumulative','prey_hp_zero_after_attack_cumulative','toxin_exposure_events_cumulative','toxin_exposure_total_cumulative')

def read(path):
 with path.open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def save(path,rows,columns):
 with path.open('w',encoding='utf-8',newline='') as f:
  w=csv.DictWriter(f,fieldnames=columns);w.writeheader();w.writerows(rows)
def number(s):
 v=float(s)
 if not math.isfinite(v):raise ValueError('non-finite numeric value')
 return v

def analyze(root,out,ticks,window,drop):
 out.mkdir(parents=True,exist_ok=True); issues=[];runs=[];intervals=[];events=[];paired=[];series={}
 for cond in CONDITIONS:
  for path in sorted(root.glob(f'{cond}-seed*.population-pressure.csv')):
   try:
    raw=read(path)
    if not raw:raise ValueError('empty file')
    seed=int(raw[0]['seed']);expected=f'{cond}-seed{seed}.population-pressure.csv'
    if path.name!=expected:raise ValueError('filename/seed mismatch')
    rows=[]
    for r in raw:
     if int(r['seed'])!=seed:raise ValueError('inconsistent seed')
     row={'tick':int(r['tick']),'herbivores':int(r['herbivores']),'plants':int(r['plants']),'carnivores':int(r['carnivores'])}
     for c in COUNTERS:row[c]=number(r[c])
     if min(row['herbivores'],row['plants'],row['carnivores'])<0:raise ValueError('negative population')
     rows.append(row)
    ts=[r['tick'] for r in rows]
    if ts[0]!=0 or ts[-1]!=ticks or ts!=sorted(set(ts)):raise ValueError(f'invalid tick range: {ts[0]}..{ts[-1]}, expected 0..{ticks}')
    for a,b in zip(rows,rows[1:]):
     for c in COUNTERS:
      if b[c]+1e-7<a[c]:raise ValueError(f'nonmonotonic counter {c} at {b["tick"]}')
    key=(seed,cond)
    if key in series:raise ValueError('duplicate seed/condition')
    series[key]=rows
    # Window-aligned observations. Sampling every 100 ticks means no interpolation is needed for standard window=1000.
    for a,b in zip(rows,rows[1:]):
     dt=b['tick']-a['tick']; avgpop=(a['herbivores']+b['herbivores'])/2
     exposure=b['toxin_exposure_events_cumulative']-a['toxin_exposure_events_cumulative']
     hunts=b['hunt_attempts_cumulative']-a['hunt_attempts_cumulative']
     deaths=b['herbivore_deaths_cumulative']-a['herbivore_deaths_cumulative']
     intervals.append({'seed':seed,'condition':cond,'start_tick':a['tick'],'end_tick':b['tick'],'herbivores_start':a['herbivores'],'herbivores_end':b['herbivores'],'herbivores_delta':b['herbivores']-a['herbivores'],'births':b['herbivore_births_cumulative']-a['herbivore_births_cumulative'],'deaths':deaths,'toxin_events':exposure,'toxin_total':b['toxin_exposure_total_cumulative']-a['toxin_exposure_total_cumulative'],'hunt_attempts':hunts,'hunt_positive':b['hunt_positive_energy_cumulative']-a['hunt_positive_energy_cumulative'],'prey_hp_zero_after_attack':b['prey_hp_zero_after_attack_cumulative']-a['prey_hp_zero_after_attack_cumulative'],'toxin_events_per_1000_herbivore_ticks':exposure/(avgpop*dt)*1000 if avgpop>0 else None,'hunt_attempts_per_1000_herbivore_ticks':hunts/(avgpop*dt)*1000 if avgpop>0 else None})
    # First observed zero is not necessarily absorbing extinction.
    zeros=[r['tick'] for r in rows if r['herbivores']==0]
    firstzero=zeros[0] if zeros else None
    # Earliest decline of at least threshold over exactly window ticks; do not interpolate missing points.
    lookup={r['tick']:r for r in rows};drops=[]
    for r in rows:
     later=lookup.get(r['tick']+window)
     if later is not None and r['herbivores']>0:
      frac=(r['herbivores']-later['herbivores'])/r['herbivores']
      if frac>=drop:drops.append((r['tick'],later['tick'],frac))
    firstdrop=drops[0] if drops else None
    # Strictly preceding window, no overlap with event interval.
    event_tick=firstdrop[0] if firstdrop else firstzero
    def preceding(end):
     start=end-window
     if start<0 or start not in lookup or end not in lookup:return None
     a,b=lookup[start],lookup[end];pop=(a['herbivores']+b['herbivores'])/2
     return {'pre_start_tick':start,'pre_end_tick':end,'pre_herbivores':a['herbivores'],'pre_to_end_herbivores':b['herbivores'],'pre_toxin_events':b['toxin_exposure_events_cumulative']-a['toxin_exposure_events_cumulative'],'pre_hunt_attempts':b['hunt_attempts_cumulative']-a['hunt_attempts_cumulative'],'pre_deaths':b['herbivore_deaths_cumulative']-a['herbivore_deaths_cumulative'],'pre_toxin_events_per_1000_herbivore_ticks':(b['toxin_exposure_events_cumulative']-a['toxin_exposure_events_cumulative'])/(pop*window)*1000 if pop>0 else None,'pre_hunt_attempts_per_1000_herbivore_ticks':(b['hunt_attempts_cumulative']-a['hunt_attempts_cumulative'])/(pop*window)*1000 if pop>0 else None}
    before=preceding(event_tick) if event_tick is not None else None
    ev={'seed':seed,'condition':cond,'first_zero_tick':firstzero,'zero_at_final':int(rows[-1]['herbivores']==0),'first_decline_start_tick':firstdrop[0] if firstdrop else None,'first_decline_end_tick':firstdrop[1] if firstdrop else None,'first_decline_fraction':firstdrop[2] if firstdrop else None,'event_reference_tick':event_tick,'pre_window_available':int(before is not None)}
    for k in ('pre_start_tick','pre_end_tick','pre_herbivores','pre_to_end_herbivores','pre_toxin_events','pre_hunt_attempts','pre_deaths','pre_toxin_events_per_1000_herbivore_ticks','pre_hunt_attempts_per_1000_herbivore_ticks'):ev[k]=before[k] if before else None
    events.append(ev)
    runs.append({'seed':seed,'condition':cond,'final_tick':ticks,'sample_count':len(rows),'first_zero_tick':firstzero,'final_zero':int(rows[-1]['herbivores']==0),'first_decline_start_tick':ev['first_decline_start_tick'],'decline_count':len(drops)})
   except (ValueError,KeyError,TypeError,IndexError) as e:issues.append(f'{path.name}: {e}')
 seeds=sorted({s for s,c in series})
 for seed in seeds:
  b=next((r for r in runs if r['seed']==seed and r['condition']==CONDITIONS[0]),None)
  f=next((r for r in runs if r['seed']==seed and r['condition']==CONDITIONS[1]),None)
  if not b or not f:issues.append(f'unpaired seed {seed}');continue
  paired.append({'seed':seed,'B_first_zero_tick':b['first_zero_tick'],'F_first_zero_tick':f['first_zero_tick'],'B_final_zero':b['final_zero'],'F_final_zero':f['final_zero'],'B_first_decline_tick':b['first_decline_start_tick'],'F_first_decline_tick':f['first_decline_start_tick'],'F_minus_B_first_decline_tick':f['first_decline_start_tick']-b['first_decline_start_tick'] if f['first_decline_start_tick'] is not None and b['first_decline_start_tick'] is not None else None})
 for cond in CONDITIONS:
  count=sum(r['condition']==cond for r in runs)
  if count!=20:issues.append(f'{cond}: {count} runs, expected 20')
 if len(paired)!=20:issues.append(f'{len(paired)} paired seeds, expected 20')
 save(out/'run_events.csv',events,list(events[0]) if events else ['seed','condition'])
 save(out/'interval_rates.csv',intervals,list(intervals[0]) if intervals else ['seed','condition'])
 save(out/'paired_event_timing.csv',paired,list(paired[0]) if paired else ['seed'])
 save(out/'run_summary.csv',runs,list(runs[0]) if runs else ['seed','condition'])
 summary={'expected_ticks':ticks,'window_ticks':window,'drop_fraction_threshold':drop,'condition_runs':len(runs),'paired_seeds':len(paired),'issues':issues,'limitations':['The first observed zero may be followed by recovery; final-zero is not proof of irreversible extinction.','Decline onset is the start of the first window meeting the threshold, not the biological onset.','Pre-event exposure is strictly before decline window, but not proof of causation.','Cumulative counters and per-capita rates have exposure-time, population, and reverse-causality limitations.','Hunt attempts and prey HP zero after attack are not confirmed predator-caused deaths.','This dataset alone does not recompute high-vs-low resistance survival or establish genetic selection.','If a sampling tick is missing, event windows are not interpolated.']}
 (out/'quality.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
 lines=['# Phase 07.2.13 — 時系列診断','',f'- 実験: {len(runs)} / 40',f'- ペアシード: {len(paired)} / 20',f'- 急減定義: {window} Tickで草食個体数が{drop:.0%}以上減少（開始時個体数 > 0）',f'- 品質問題: {len(issues)}件','','## 条件別の観測']
 for cond in CONDITIONS:
  rr=[r for r in runs if r['condition']==cond]
  lines.append(f'- {cond}: 最終時点ゼロ {sum(r["final_zero"] for r in rr)} / {len(rr)}、急減検出 {sum(r["first_decline_start_tick"] is not None for r in rr)} / {len(rr)}')
 lines+=['','## 解釈上の注意','- 急減直前の毒曝露・捕食試行を記録するが、因果関係は特定できない。','- 急減の閾値は診断用の事前固定値。感度分析は別途行い、事後選択に注意。','- 生存率差の主要仮説検定とは別の探索的分析。','', '## 品質問題']
 lines+=['- '+x for x in issues] if issues else ['- なし']
 (out/'REPORT.md').write_text('\n'.join(lines)+'\n',encoding='utf-8')
 return summary

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--root',default='results/phase07212/runs');ap.add_argument('--out',default='results/phase07213/analysis');ap.add_argument('--expected-ticks',type=int,default=50000);ap.add_argument('--window',type=int,default=1000);ap.add_argument('--drop-fraction',type=float,default=.30);a=ap.parse_args()
 if a.expected_ticks<1 or a.window<1 or not 0<a.drop_fraction<=1:ap.error('invalid duration/window/threshold')
 result=analyze(Path(a.root),Path(a.out),a.expected_ticks,a.window,a.drop_fraction)
 print(f'Runs={result["condition_runs"]} paired={result["paired_seeds"]} issues={len(result["issues"])}')
 if result['issues']:raise SystemExit(2)
if __name__=='__main__':main()
