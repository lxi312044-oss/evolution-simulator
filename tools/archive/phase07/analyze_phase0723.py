#!/usr/bin/env python3
"""Phase 07.2.3: cohort-matched observational selection diagnostics.
No change to simulation model; no individual-level p-values (pseudo-replication).
"""
import argparse,csv,collections,statistics,math
from pathlib import Path
CONDS=['A_no_toxin_no_cost','B_no_toxin_cost','C_toxin_no_cost','D_toxin_cost','E_strong_no_cost','F_strong_cost']

def read(p):
 with p.open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def val(x,default=0):
 try:return float(x)
 except (ValueError,TypeError):return default
def write(p,records,fields):
 p.parent.mkdir(parents=True,exist_ok=True)
 with p.open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(records)
def analyze(root,out,window,width):
 cohorts=[];runs=[];trends=[];extinctions=[];qc=[]
 for cond in CONDS:
  for gp in sorted((root/cond).glob('seed-*.genealogy.csv')):
   seed=gp.name.split('.')[0].split('-')[-1]
   rp=gp.with_name(gp.name.replace('.genealogy.csv','.resistance.csv'))
   if not rp.exists():raise FileNotFoundError(rp)
   g=read(gp);r=read(rp)
   if not r:raise ValueError(f'empty resistance CSV: {rp}')
   end=int(val(r[-1]['tick']))
   if end<window:raise ValueError(f'run too short: {gp}')
   people={};childtimes=collections.defaultdict(list);birthcount=0
   for e in g:
    key=(e['id_index'],e['id_generation']);kind=e['event'];tick=int(val(e['tick']))
    if kind in ('founder','birth'):
     if key in people:raise ValueError(f'duplicate individual {key} {gp}')
     people[key]={'born':tick,'res':val(e['child_resistance']),'death':None,'kind':kind}
     if kind=='birth':
      birthcount+=1;parent=(e['parent_id_index'],e['parent_id_generation'])
      childtimes[parent].append(tick)
    elif kind=='death':
     if key not in people:raise ValueError(f'unknown death {key} {gp}')
     people[key]['death']=tick
   missingparents=[k for k in childtimes if k not in people]
   if missingparents:raise ValueError(f'unknown parents: {len(missingparents)} {gp}')
   qc.append(dict(condition=cond,seed=seed,people=len(people),births=birthcount,missing_parents=0,final_tick=end))
   # Cohort bins and resistance bins, with fixed complete followup; keep each seed independent.
   groups=collections.defaultdict(list)
   for key,p in people.items():
    born=p['born'];res=p['res']
    if born+window>end:continue
    b='low' if res<.49 else ('high' if res>.51 else 'mid')
    survived=p['death'] is None or p['death']>born+window
    children=sum(born<=t<born+window for t in childtimes.get(key,[]))
    groups[(born//width,b)].append((int(survived),children,res))
   for (cohort,b),members in sorted(groups.items()):
    cohorts.append(dict(condition=cond,seed=seed,cohort_start=cohort*width,resistance_bin=b,n=len(members),survived=sum(x[0] for x in members),offspring=sum(x[1] for x in members),survival_rate=sum(x[0] for x in members)/len(members),offspring_per_individual=sum(x[1] for x in members)/len(members)))
   # Sample trajectory: never treat extinct mean=0 as actual gene mean.
   for x in r:
    pop=int(val(x['population']))
    trends.append(dict(condition=cond,seed=seed,tick=int(val(x['tick'])),population=pop,mean_resistance=x['mean'] if pop else '',mean_energy=x.get('energy_mean',''),resistance_cost=x.get('cumulative_resistance_cost',''),prevented_damage=x.get('cumulative_prevented_damage','')))
   firstzero=next((i for i,x in enumerate(r) if i>0 and val(x['population'])==0),None)
   if firstzero is not None:
    for x in r[max(0,firstzero-10):firstzero+1]:
     extinctions.append(dict(condition=cond,seed=seed,extinction_tick=r[firstzero]['tick'],tick=x['tick'],population=x['population'],mean_resistance=x['mean'] if val(x['population']) else '',mean_energy=x.get('energy_mean',''),resistance_cost=x.get('cumulative_resistance_cost',''),toxin_damage=x.get('cumulative_toxin_damage','')))
   runs.append(dict(condition=cond,seed=seed,final_tick=end,final_population=r[-1]['population'],initial_mean=r[0]['mean'],final_mean=r[-1]['mean'] if val(r[-1]['population']) else '',births=birthcount,extinct=int(firstzero is not None)))
 # Matched within-seed, within-birth-cohort low/high contrasts; never pool individuals as independent replicates.
 by=collections.defaultdict(dict)
 for c in cohorts:by[(c['condition'],c['seed'],c['cohort_start'])][c['resistance_bin']]=c
 contrasts=[]
 for (cond,seed,cohort),bins in sorted(by.items()):
  if 'low' not in bins or 'high' not in bins:continue
  lo,hi=bins['low'],bins['high']
  if min(lo['n'],hi['n'])<5:continue
  contrasts.append(dict(condition=cond,seed=seed,cohort_start=cohort,n_low=lo['n'],n_high=hi['n'],survival_diff_high_minus_low=hi['survival_rate']-lo['survival_rate'],offspring_diff_high_minus_low=hi['offspring_per_individual']-lo['offspring_per_individual']))
 seedcontrasts=[]
 for cond in CONDS:
  for seed in sorted({x['seed'] for x in runs if x['condition']==cond}):
   rows=[x for x in contrasts if x['condition']==cond and x['seed']==seed]
   if not rows:continue
   seedcontrasts.append(dict(condition=cond,seed=seed,matched_cohorts=len(rows),mean_survival_diff=statistics.mean(x['survival_diff_high_minus_low'] for x in rows),mean_offspring_diff=statistics.mean(x['offspring_diff_high_minus_low'] for x in rows)))
 # Paired environment differences at seed level, not per individual.
 pairings=[('B_no_toxin_cost','A_no_toxin_no_cost'),('C_toxin_no_cost','A_no_toxin_no_cost'),('D_toxin_cost','C_toxin_no_cost'),('E_strong_no_cost','C_toxin_no_cost'),('F_strong_cost','E_strong_no_cost')]
 lookup={(x['condition'],x['seed']):x for x in seedcontrasts};paired=[]
 for treatment,control in pairings:
  for seed in sorted({x['seed'] for x in seedcontrasts}):
   a=lookup.get((treatment,seed));b=lookup.get((control,seed))
   if a and b:paired.append(dict(treatment=treatment,control=control,seed=seed,offspring_selection_difference= a['mean_offspring_diff']-b['mean_offspring_diff'],survival_selection_difference=a['mean_survival_diff']-b['mean_survival_diff']))
 write(out/'run_summary.csv',runs,['condition','seed','final_tick','final_population','initial_mean','final_mean','births','extinct'])
 write(out/'cohort_stats.csv',cohorts,['condition','seed','cohort_start','resistance_bin','n','survived','offspring','survival_rate','offspring_per_individual'])
 write(out/'matched_cohort_contrasts.csv',contrasts,['condition','seed','cohort_start','n_low','n_high','survival_diff_high_minus_low','offspring_diff_high_minus_low'])
 write(out/'seed_selection_contrasts.csv',seedcontrasts,['condition','seed','matched_cohorts','mean_survival_diff','mean_offspring_diff'])
 write(out/'paired_condition_contrasts.csv',paired,['treatment','control','seed','offspring_selection_difference','survival_selection_difference'])
 write(out/'resistance_trajectory.csv',trends,['condition','seed','tick','population','mean_resistance','mean_energy','resistance_cost','prevented_damage'])
 write(out/'pre_extinction.csv',extinctions,['condition','seed','extinction_tick','tick','population','mean_resistance','mean_energy','resistance_cost','toxin_damage'])
 write(out/'data_quality.csv',qc,['condition','seed','people','births','missing_parents','final_tick'])
 report=['# Phase 07.2.3 自然選択の統計的検証（観測分析）','',f'- 完走データ: {len(runs)} 件',f'- 親子整合性チェック: {len(qc)} 件',f'- 比較可能な出生コホート: {len(contrasts)} 件',f'- シード内の比較: {len(seedcontrasts)} 件',f'- 対応シード条件差: {len(paired)} 件','','## 判定上の注意','- 個体を独立した反復実験とみなさない。主な独立単位はシード。','- 低耐性と高耐性が同じ出生コホートに各5体以上いる場合のみ比較する。','- 耐性値の分布が狭く比較不能な場合は「検出力不足」であり、選択なしの証明ではない。','- 出生コホートで揃えても、血縁・資源・曝露・空間条件による交絡は残る。','- 絶滅シードを除外せず記録する。絶滅後の平均耐性は欠測とする。','- 5シードで有意差が出ないことは効果なしを意味しない。','','## シード単位の高耐性−低耐性差（記述統計）','']
 for cond in CONDS:
  rs=[x for x in seedcontrasts if x['condition']==cond]
  if rs:report.append(f"- {cond}: {len(rs)}シード、出生数差平均 {statistics.mean(x['mean_offspring_diff'] for x in rs):+.5f}、生存率差平均 {statistics.mean(x['mean_survival_diff'] for x in rs):+.5f}")
  else:report.append(f'- {cond}: 比較可能な低・高耐性コホートなし（判定保留）')
 (out/'REPORT.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
 print(f'Analyzed {len(runs)} runs, {len(contrasts)} matched cohorts, {len(seedcontrasts)} seed contrasts, {len(paired)} paired condition comparisons.')
 print('Read',out/'REPORT.md')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--root',type=Path,default=Path('results/phase0722'));p.add_argument('--out',type=Path,default=Path('results/phase0723/analysis'));p.add_argument('--window',type=int,default=1000);p.add_argument('--cohort-width',type=int,default=1000)
 a=p.parse_args();analyze(a.root,a.out,a.window,a.cohort_width)
