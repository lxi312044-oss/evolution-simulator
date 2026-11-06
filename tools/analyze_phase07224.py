#!/usr/bin/env python3
"""Phase 07.2.24: resistance-selection mechanism diagnostics, observational only."""
import argparse,csv,math,json,statistics
from collections import defaultdict
from pathlib import Path

def emit(path,rows,fields):
 with path.open('w',encoding='utf-8',newline='') as f:
  w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(rows)
def mean(x):return statistics.mean(x) if x else None
def fmt(v):return 'NA' if v is None else f'{v:.5f}'
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--input',required=True);ap.add_argument('--out',default='results/phase07224/analysis');ap.add_argument('--ticks',type=int,default=50000);args=ap.parse_args()
 out=Path(args.out);out.mkdir(parents=True,exist_ok=True)
 grouped=defaultdict(list);issues=[];seen=set();counts=defaultdict(int)
 with open(args.input,encoding='utf-8-sig',newline='') as f:
  for line,r in enumerate(csv.DictReader(f),2):
   try:
    level=r['level'];seed=int(r['seed']);identity=(level,seed,r['id']);
    if identity in seen:raise ValueError('duplicate individual ID')
    seen.add(identity)
    origin=int(r['origin_tick']);end=int(r['terminal_tick']) if r['terminal_tick'] else None
    resistance=float(r['resistance_at_origin']);children=int(r['observed_children']);age0=int(r['age_at_origin'])
    if not (0<=resistance<=1 and 0<=origin<=args.ticks and children>=0 and age0>=0):raise ValueError('out-of-range field')
    if end is not None and not(origin<=end<=args.ticks):raise ValueError('invalid terminal tick')
    dose=float(r['toxin_dose_at_terminal']) if r['toxin_dose_at_terminal'] else None
    damage=float(r['toxin_damage_at_terminal']) if r['toxin_damage_at_terminal'] else None
    cost=float(r['resistance_cost_at_terminal']) if r['resistance_cost_at_terminal'] else None
    for v in (dose,damage,cost):
     if v is not None and (not math.isfinite(v) or v<0):raise ValueError('invalid cumulative value')
    if r['origin']!='birth':continue
    if r['status'] not in ('observed_disappearance','unobserved_terminal'):counts['unknown_status']+=1
    grouped[(level,seed)].append(dict(res=resistance,children=children,origin=origin,end=end,status=r['status'],dose=dose,damage=damage,cost=cost,followup=(end if end is not None else args.ticks)-origin))
   except (ValueError,KeyError) as e:issues.append(f'line {line}: {e}')
 if issues:raise ValueError('Invalid input: '+str(issues[:10]))
 rows=[];groups=defaultdict(list)
 for (level,seed),individuals in sorted(grouped.items()):
  for cohort in range(0,args.ticks,5000):
   # Match both resistance bands within each seed and 5k birth cohort.
   subset=[x for x in individuals if cohort<=x['origin']<cohort+5000]
   for band,items in [('low',[x for x in subset if x['res']<0.45]),('mid',[x for x in subset if 0.45<=x['res']<=0.55]),('high',[x for x in subset if x['res']>0.55])]:
    if not items:continue
    finished=[x for x in items if x['status']=='observed_disappearance']
    # Fixed-horizon endpoints avoid equating end-of-run censoring with survival.
    eligible=[x for x in items if x['origin']<=args.ticks-1000]
    completed1000=[x for x in eligible if x['status']=='observed_disappearance' and x['end'] is not None and x['end']-x['origin']<1000]
    exposure=[x for x in finished if x['dose'] is not None]
    row=dict(level=level,seed=seed,cohort_start=cohort,band=band,n=len(items),completed=len(finished),unobserved=len(items)-len(finished),mean_resistance=mean([x['res'] for x in items]),mean_children=mean([x['children'] for x in items]),mean_followup_ticks=mean([x['followup'] for x in items]),n_eligible_1000=len(eligible),disappeared_within_1000=len(completed1000),disappearance_1000_rate=len(completed1000)/len(eligible) if eligible else None,mean_terminal_dose_completed=mean([x['dose'] for x in exposure]),mean_terminal_damage_completed=mean([x['damage'] for x in finished if x['damage'] is not None]),mean_terminal_cost_completed=mean([x['cost'] for x in finished if x['cost'] is not None]))
    rows.append(row)
  groups[level].append((seed,individuals))
 fields=list(rows[0]) if rows else []
 if rows:emit(out/'cohort_mechanisms.csv',rows,fields)
 comparisons=[]
 for level in ['control','x1','x2','x4']:
  for seed,items in groups[level]:
   # compare within matched cohort and seed; do not pool individuals as independent runs
   buckets=defaultdict(lambda:defaultdict(list))
   for x in items:
    b='low' if x['res']<.45 else ('high' if x['res']>.55 else 'mid')
    buckets[x['origin']//5000][b].append(x)
   for cohort,byband in buckets.items():
    low,high=byband['low'],byband['high']
    if min(len(low),len(high))<10:continue
    eligible_low=[x for x in low if x['origin']<=args.ticks-1000]
    eligible_high=[x for x in high if x['origin']<=args.ticks-1000]
    def early_rate(arr):
     return sum(x['status']=='observed_disappearance' and x['end'] is not None and x['end']-x['origin']<1000 for x in arr)/len(arr) if arr else None
    def diff(a,b):return a-b if a is not None and b is not None else None
    comparisons.append(dict(level=level,seed=seed,cohort_start=cohort*5000,n_low=len(low),n_high=len(high),high_minus_low_children=diff(mean([x['children'] for x in high]),mean([x['children'] for x in low])),high_minus_low_early_disappearance=diff(early_rate(eligible_high),early_rate(eligible_low)),high_minus_low_terminal_damage_completed=diff(mean([x['damage'] for x in high if x['status']=='observed_disappearance' and x['damage'] is not None]),mean([x['damage'] for x in low if x['status']=='observed_disappearance' and x['damage'] is not None])),high_minus_low_terminal_cost_completed=diff(mean([x['cost'] for x in high if x['status']=='observed_disappearance' and x['cost'] is not None]),mean([x['cost'] for x in low if x['status']=='observed_disappearance' and x['cost'] is not None]))))
 if comparisons:emit(out/'matched_cohort_differences.csv',comparisons,list(comparisons[0]))
 summary=[]
 for level in ['control','x1','x2','x4']:
  cr=[r for r in comparisons if r['level']==level]
  seeds=defaultdict(list)
  for r in cr:seeds[r['seed']].append(r)
  seed_results=[]
  for seed,rs in seeds.items():
   entry=dict(level=level,seed=seed,cohorts=len(rs))
   for field in ['high_minus_low_children','high_minus_low_early_disappearance','high_minus_low_terminal_damage_completed','high_minus_low_terminal_cost_completed']:
    entry[field]=mean([r[field] for r in rs if r[field] is not None])
   seed_results.append(entry)
  summary+=seed_results
 if summary:emit(out/'run_differences.csv',summary,list(summary[0]))
 report=['# Phase 07.2.24 — 毒耐性の選択圧が弱い原因の診断','',f'入力: {args.input}',f'解析ラン: {len(grouped)}、出生個体: {sum(map(len,grouped.values())):,}','', '## 同一seed・出生時期内の高耐性群と低耐性群の比較','', '|条件|比較可能seed|出生子数差 (高−低)|1000tick以内の消失率差|累積毒損傷差 (消失個体のみ)|耐性コスト差 (消失個体のみ)|','|---|---:|---:|---:|---:|---:|']
 for level in ['control','x1','x2','x4']:
  r=[x for x in summary if x['level']==level]
  report.append('|'+ '|'.join([level,str(len(r))]+[fmt(mean([x[f] for x in r if x[f] is not None])) for f in ['high_minus_low_children','high_minus_low_early_disappearance','high_minus_low_terminal_damage_completed','high_minus_low_terminal_cost_completed']])+'|')
 report+=['','## 解釈上の制約','', '- 高耐性群と低耐性群の観察的比較であり、耐性の因果効果を確定できない。','- 1000tick以内の消失は死因別の死亡ではない。','- 累積毒損傷・コストは消失個体のみで比較しており、生存時間や摂食量の差が混入する。','- 高・低耐性群の人数が各出生コホート10未満の場合は比較から除外する。','- 親子の遺伝的選択の証明には世代別耐性分布・親子形質差の別途解析が必要。','- 80ランの検証が前回と同じ生態条件だったかは、元の実行設定と突き合わせて確認する必要がある。','',f'入力検証エラー: {len(issues)}','']
 (out/'REPORT.md').write_text('\n'.join(report),encoding='utf-8')
 (out/'quality.json').write_text(json.dumps(dict(runs=len(grouped),individuals=sum(map(len,grouped.values())),matched_cohorts=len(comparisons),input_errors=issues,limitations=report[-8:-2]),indent=2,ensure_ascii=False),encoding='utf-8')
 print('\n'.join(report))
if __name__=='__main__':main()
