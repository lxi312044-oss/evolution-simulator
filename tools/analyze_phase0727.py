#!/usr/bin/env python3
"""Fixed-horizon, birth-cohort matched descriptive analysis. Standard library only."""
import argparse,csv,collections,statistics
from pathlib import Path

def main():
 p=argparse.ArgumentParser();p.add_argument('--root',default='results/phase0727');p.add_argument('--out',default='results/phase0727/analysis');p.add_argument('--horizon',type=int,default=1000);p.add_argument('--cohort-width',type=int,default=5000);a=p.parse_args()
 if a.horizon<=0 or a.cohort_width<=0:p.error('horizon and cohort-width must be positive')
 root=Path(a.root);out=Path(a.out);out.mkdir(parents=True,exist_ok=True)
 files=sorted(root.glob('*.individual-events.csv'))
 if not files:raise SystemExit('No event CSV files: '+str(root))
 rows=[];quality=[]
 for f in files:
  run=f.name.removesuffix('.individual-events.csv'); individuals={}; last_tick=0; counts=collections.Counter()
  with f.open(encoding='utf-8-sig',newline='') as fp:
   reader=csv.DictReader(fp)
   required={'tick','event','id_index','id_generation','resistance','amount'}
   if not required.issubset(reader.fieldnames or []):raise ValueError(f'{f}: missing required columns')
   for line,r in enumerate(reader,2):
    try:
     tick=int(r['tick']);key=(r['id_index'],r['id_generation']);event=r['event'];value=float(r['amount']);res=float(r['resistance']);last_tick=max(last_tick,tick)
     if value<0:raise ValueError('negative amount')
     if event=='birth':
      if key in individuals:raise ValueError('duplicate birth')
      individuals[key]={'born':tick,'res':res,'death':None,'events':[]}
     elif event=='reproduction':
      # Parent's reproductive event is emitted at the same tick as child's birth.
      if key in individuals:individuals[key]['events'].append((tick,event,value))
      # Founders lack a known birth tick and are excluded from cohort comparisons.
     elif event=='death':
      if key in individuals:individuals[key]['death']=tick
     elif event=='censored':pass
     elif event in ('toxin_dose','toxin_damage','prevented_damage','resistance_cost'):
      if key in individuals:individuals[key]['events'].append((tick,event,value))
     else:raise ValueError('unknown event '+event)
     counts[event]+=1
    except (ValueError,TypeError) as e:raise ValueError(f'{f}:{line}: {e}') from e
  # Only cohorts with complete observation windows are eligible.
  cohorts=collections.defaultdict(lambda:collections.defaultdict(list))
  for key,d in individuals.items():
   born=d['born']; end=born+a.horizon
   if end>last_tick:continue
   if d['res']<.49:group='low'
   elif d['res']>.51:group='high'
   else:continue
   death=d['death']; survived=death is None or death>=end
   metrics=collections.Counter()
   for tick,event,value in d['events']:
    if born<tick<=end and (death is None or tick<=death):metrics[event]+=value
   cohorts[born//a.cohort_width][group].append((int(survived),metrics))
  for cohort,groups in sorted(cohorts.items()):
   if not groups['low'] or not groups['high']:continue
   def avg(g,key):return sum(x[0] if key=='survival' else x[1][key] for x in g)/len(g)
   row={'run':run,'cohort':cohort,'n_low':len(groups['low']),'n_high':len(groups['high'])}
   for metric in ('survival','reproduction','toxin_dose','toxin_damage','prevented_damage','resistance_cost'):
    lo=avg(groups['low'],metric);hi=avg(groups['high'],metric)
    row[metric+'_low']=lo;row[metric+'_high']=hi;row[metric+'_diff']=hi-lo
   rows.append(row)
  if not counts['birth']:quality.append((run,'no births observed'))
  if not counts['death'] and not counts['censored']:quality.append((run,'no follow-up endpoints'))
 fields=['run','cohort','n_low','n_high']+[m+'_'+suffix for m in ('survival','reproduction','toxin_dose','toxin_damage','prevented_damage','resistance_cost') for suffix in ('low','high','diff')]
 with (out/'cohort_comparisons.csv').open('w',encoding='utf-8',newline='') as fp:
  writer=csv.DictWriter(fp,fieldnames=fields);writer.writeheader();writer.writerows(rows)
 bycondition=collections.defaultdict(lambda:collections.defaultdict(list))
 byrun=collections.defaultdict(list)
 for r in rows:byrun[r['run']].append(r)
 for run,rr in byrun.items():
  weights=[min(x['n_low'],x['n_high']) for x in rr]
  if not sum(weights):continue
  condition=run.split('-seed')[0]
  for metric in ('survival','reproduction','toxin_dose','resistance_cost'):
   bycondition[condition][metric].append(sum(x[metric+'_diff']*w for x,w in zip(rr,weights))/sum(weights))
 report=['# Phase 07.2.7 — 個体別イベント時系列解析','',f'- 入力ファイル数: {len(files)}',f'- 比較可能なコホート: {len(rows)}',f'- 観測期間: 出生後{a.horizon} Tick',f'- 出生コホート幅: {a.cohort_width} Tick','- 耐性区分: low < 0.49 / high > 0.51','- 差は高耐性−低耐性。コホート内比較を少人数側の人数で加重し、シード別に算出。','','| 条件 | シード数 | 生存差 | 繁殖数差 | 毒曝露差 | 維持コスト差 |','|---|---:|---:|---:|---:|---:|']
 for cond,metrics in sorted(bycondition.items()):
  n=len(metrics['survival']);fmt=lambda m: f'{statistics.mean(metrics[m]):+.6f}' if metrics[m] else '—'
  report.append(f'| {cond} | {n} | {fmt("survival")} | {fmt("reproduction")} | {fmt("toxin_dose")} | {fmt("resistance_cost")} |')
 report+=['','## 制約と注意','- これは観測データの記述統計であり、自然選択の因果証明ではない。','- 個体の毒曝露量は捕食行動・生存期間の影響を受けるため、単純な回帰による因果推論は避ける。','- 創始個体は出生時刻が不明なのでコホート比較から除外。','- 死亡が発生したTickの曝露・コストは死亡前に記録された場合のみ含む。','- 生存判定は観測期間の末端まで生存したかで判定。','- シードを独立反復単位とし、個体数を独立反復数として扱わない。','- 既存のPhase 07.2.5データだけからこのイベント時系列を復元することはできない。新たな実験が必要。','- 生態ロジックに変更なし。','',f'## 品質警告: {len(quality)}']
 report.extend(f'- {run}: {issue}' for run,issue in quality)
 if not rows:report.append('- 比較可能なコホートなし。観測期間や実験長を確認。')
 (out/'REPORT.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
 print(f'files={len(files)} matched_cohorts={len(rows)} warnings={len(quality)} output={out}')
if __name__=='__main__':main()
