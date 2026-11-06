#!/usr/bin/env python3
"""Phase 07.2.16: temporal diagnostic; observational, not causal."""
import argparse,csv,json,math,pathlib,statistics,sys
from collections import defaultdict

def read(path):
 with open(path,encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def write(path,rows):
 if not rows:return
 with open(path,'w',encoding='utf-8',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
def num(row,key):return float(row[key])
def main():
 p=argparse.ArgumentParser();p.add_argument('--root',default='results/phase07215/runs');p.add_argument('--intervals',default='results/phase07213/analysis/interval_rates.csv');p.add_argument('--out',default='results/phase07216/analysis');p.add_argument('--expected-ticks',type=int,default=50000);p.add_argument('--start',type=int,default=2500);p.add_argument('--end',type=int,default=6500);a=p.parse_args()
 out=pathlib.Path(a.out);out.mkdir(parents=True,exist_ok=True);issues=[];runs={};detail=[];lag=[];intervals={}
 files=sorted(pathlib.Path(a.root).glob('*.reproduction.csv'))
 for f in files:
  key=f.name.removesuffix('.reproduction.csv'); condition,seedstr=key.rsplit('-seed',1);seed=int(seedstr);rows=read(f)
  if not rows:issues.append(f'empty {key}');continue
  ticks=[int(r['tick']) for r in rows]
  if ticks[0]!=0 or ticks[-1]!=a.expected_ticks or len(ticks)!=len(set(ticks)) or any(y-x!=100 for x,y in zip(ticks,ticks[1:])):issues.append(f'invalid tick coverage {key}: {ticks[0]}..{ticks[-1]}')
  runs[(condition,seed)]=rows
  prev=None
  for r in rows:
   tick=int(r['tick'])
   if tick<a.start or tick>a.end or tick==0:continue
   d={'condition':condition,'seed':seed,'tick':tick,'start_tick':tick-100,'end_tick':tick,'population':int(r['herbivores'])}
   for col in ('herbivore_births_delta','herbivore_reproduction_attempts_delta','phase24_repro_samples_delta','phase24_repro_energy_ready_delta','phase24_repro_satiety_ready_delta','phase24_repro_ready_all_delta','phase20_repro_energy_gate_blocked_delta','phase20_repro_satiety_blocked_delta','herbivore_hp_deaths_delta','herbivore_age_deaths_delta','toxin_exposure_events_delta'):
    d[col]=num(r,col)
   den=d['phase24_repro_samples_delta'];d['energy_ready_rate']=d['phase24_repro_energy_ready_delta']/den if den else None;d['satiety_ready_rate']=d['phase24_repro_satiety_ready_delta']/den if den else None
   detail.append(d)
 if len(runs)!=40:issues.append(f'expected 40 runs, found {len(runs)}')
 for condition in ('B_no_toxin_cost','F_strong_cost'):
  seeds={seed for c,seed in runs if c==condition}
  if len(seeds)!=20:issues.append(f'{condition}: expected 20 seeds, got {len(seeds)}')
 if pathlib.Path(a.intervals).exists():
  for r in read(a.intervals):
   k=(r['condition'],int(r['seed']),int(r['end_tick']))
   intervals[k]=r
  if len(intervals)<40*40:issues.append('interval rates unexpectedly sparse')
 else:issues.append('phase07213 interval rates unavailable; plant and predation fields omitted')
 for d in detail:
  r=intervals.get((d['condition'],d['seed'],d['tick']))
  if r:
   d['plant_data_available']=False # phase13 intervals have no plant stock
   d['hunt_attempts']=num(r,'hunt_attempts');d['toxin_events_phase13']=num(r,'toxin_events');d['deaths_phase13']=num(r,'deaths')
  else:
   d['plant_data_available']=False;d['hunt_attempts']='';d['toxin_events_phase13']='';d['deaths_phase13']=''
 write(out/'time_aligned_100tick.csv',detail)
 grouped=defaultdict(list)
 for d in detail:grouped[(d['condition'],d['tick'])].append(d)
 summary=[]
 for (c,t),rs in sorted(grouped.items()):
  row={'condition':c,'tick':t,'seeds':len(rs)}
  for col in ('population','herbivore_births_delta','herbivore_hp_deaths_delta','herbivore_age_deaths_delta','energy_ready_rate','satiety_ready_rate','toxin_exposure_events_delta','hunt_attempts'):
   vals=[float(r[col]) for r in rs if r[col] is not None and r[col]!=''];row['mean_'+col]=round(statistics.mean(vals),7) if vals else ''
  summary.append(row)
 write(out/'condition_time_means.csv',summary)
 # fixed 100-tick lag: covariates at interval t vs births in immediately following interval
 bykey={(d['condition'],d['seed'],d['tick']):d for d in detail}
 for d in detail:
  nxt=bykey.get((d['condition'],d['seed'],d['tick']+100))
  if not nxt:continue
  lag.append({'condition':d['condition'],'seed':d['seed'],'exposure_end_tick':d['tick'],'birth_interval_end_tick':nxt['tick'],'energy_ready_rate':d['energy_ready_rate'],'satiety_ready_rate':d['satiety_ready_rate'],'population':d['population'],'next_births':nxt['herbivore_births_delta'],'next_hp_deaths':nxt['herbivore_hp_deaths_delta'],'next_population':nxt['population']})
 write(out/'lag_100tick.csv',lag)
 # zero-birth onset after t=2500, report descriptive temporal ordering
 events=[]
 for (c,s),rows in sorted(runs.items()):
  subset=[r for r in rows if a.start<int(r['tick'])<=a.end]
  zero=next((int(r['tick']) for r in subset if num(r,'herbivore_births_delta')==0),None)
  energy=next((int(r['tick']) for r in subset if num(r,'phase24_repro_samples_delta')>0 and num(r,'phase24_repro_energy_ready_delta')/num(r,'phase24_repro_samples_delta')<0.05),None)
  satiety=next((int(r['tick']) for r in subset if num(r,'phase24_repro_samples_delta')>0 and num(r,'phase24_repro_satiety_ready_delta')/num(r,'phase24_repro_samples_delta')<0.05),None)
  events.append({'condition':c,'seed':s,'first_zero_birth_tick':zero,'first_energy_ready_below_5pct_tick':energy,'first_satiety_ready_below_5pct_tick':satiety,'energy_minus_zero_birth_tick':energy-zero if energy is not None and zero is not None else '', 'satiety_minus_zero_birth_tick':satiety-zero if satiety is not None and zero is not None else ''})
 write(out/'onset_timing.csv',events)
 quality={'condition_runs':len(runs),'paired_seeds':len({s for c,s in runs if c=='B_no_toxin_cost'}&{s for c,s in runs if c=='F_strong_cost'}),'expected_ticks':a.expected_ticks,'time_range':[a.start,a.end],'rows_100tick':len(detail),'lag_rows':len(lag),'issues':issues,'limitations':['No plant/soil resource quantities in supplied Phase07.2.15 reproduction CSV or Phase07.2.13 interval_rates CSV','Phase07.2.13 and Phase07.2.15 are separate experiment runs; matching seeds does not prove identical trajectories','Ready rates are per diagnostic sample, not independent individuals','Time precedence is not causal identification','Threshold 5% and first zero-birth interval are exploratory']}
 (out/'quality.json').write_text(json.dumps(quality,ensure_ascii=False,indent=2),encoding='utf-8')
 def avg(c,key):
  vals=[r[key] for r in detail if r['condition']==c and isinstance(r[key],(float,int))];return statistics.mean(vals) if vals else float('nan')
 report=f'''# Phase 07.2.16 — 100 Tick時系列診断\n\n## 入力・品質\n- 観測対象: {len(runs)}条件別ラン、{quality['paired_seeds']}ペア、{a.start}–{a.end} Tick\n- 100 Tick集計行数: {len(detail)}、1区間先行比較: {len(lag)}\n- 品質指摘: {len(issues)}件\n\n## 観測期間の平均（個体×Tick診断の成立率）\n| 条件 | エネルギー成立率 | 満腹成立率 | 100 Tick出生数平均 |\n|---|---:|---:|---:|\n| B | {avg('B_no_toxin_cost','energy_ready_rate'):.3%} | {avg('B_no_toxin_cost','satiety_ready_rate'):.3%} | {avg('B_no_toxin_cost','herbivore_births_delta'):.2f} |\n| F | {avg('F_strong_cost','energy_ready_rate'):.3%} | {avg('F_strong_cost','satiety_ready_rate'):.3%} | {avg('F_strong_cost','herbivore_births_delta'):.2f} |\n\n## 解釈上の制限\n- 既存の共有データには植物個体数・土壌栄養量・植物の可食資源量が100 Tick単位で揃っていません。**資源不足が直接原因かは未判定**です。\n- 先行・後続の100 Tick区間を対比しますが、共通要因や個体数変化による交絡を排除できません。\n- `phase24_repro_*`は診断観測、`phase20_repro_*`は実際の処理段階に由来し、単純に相互置換できません。\n- Phase07.2.13と07.2.15は別実験の集計であり、同じseedでも完全に同じ軌跡とは限りません。\n- 解析は観測上の前後関係の記述に限り、因果関係の証明ではありません。\n\n## 出力\n`time_aligned_100tick.csv`、`condition_time_means.csv`、`lag_100tick.csv`、`onset_timing.csv`、`quality.json`。\n'''
 (out/'REPORT.md').write_text(report,encoding='utf-8');print(f"Runs={len(runs)} pairs={quality['paired_seeds']} issues={len(issues)} rows={len(detail)}")
 return 2 if issues else 0
if __name__=='__main__':sys.exit(main())
