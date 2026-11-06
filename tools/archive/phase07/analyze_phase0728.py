#!/usr/bin/env python3
"""Phase 07.2.8: seed-level uncertainty and confounding audit; standard library only."""
import argparse,csv,collections,random,math,statistics,hashlib,json
from pathlib import Path
METRICS=['survival','reproduction','toxin_dose','toxin_damage','prevented_damage','resistance_cost']
CONDITIONS=['A_no_toxin_no_cost','B_no_toxin_cost','C_toxin_no_cost','D_toxin_cost','E_strong_no_cost','F_strong_cost']
PAIRS=[('B-A','B_no_toxin_cost','A_no_toxin_no_cost'),('C-A','C_toxin_no_cost','A_no_toxin_no_cost'),('D-B','D_toxin_cost','B_no_toxin_cost'),('E-A','E_strong_no_cost','A_no_toxin_no_cost'),('F-B','F_strong_cost','B_no_toxin_cost'),('D-C','D_toxin_cost','C_toxin_no_cost'),('F-E','F_strong_cost','E_strong_no_cost')]
def write(path,rows,fields):
 with path.open('w',encoding='utf-8',newline='') as f:
  w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(rows)
def bootstrap(values,rng,reps=5000):
 n=len(values)
 if n<2:return (None,None)
 means=sorted(sum(values[rng.randrange(n)] for _ in range(n))/n for _ in range(reps))
 return (means[int(.025*reps)],means[int(.975*reps)])
def fmt(x):return 'NA' if x is None else f'{x:+.6f}'
def main():
 p=argparse.ArgumentParser();p.add_argument('--input',default='results/phase0727/analysis/cohort_comparisons.csv');p.add_argument('--out',default='results/phase0728/analysis');p.add_argument('--bootstrap',type=int,default=5000);a=p.parse_args()
 if a.bootstrap<100:p.error('bootstrap must be >=100')
 source=Path(a.input);out=Path(a.out);out.mkdir(parents=True,exist_ok=True)
 rows=[];issues=[];byrun=collections.defaultdict(list)
 with source.open(encoding='utf-8-sig',newline='') as f:
  reader=csv.DictReader(f);need={'run','cohort','n_low','n_high'}|{m+'_'+suffix for m in METRICS for suffix in ['low','high','diff']}
  if not need.issubset(reader.fieldnames or []):raise SystemExit('Missing columns: '+str(sorted(need-set(reader.fieldnames or []))))
  for line,r in enumerate(reader,2):
   try:
    for field in ['cohort','n_low','n_high']:r[field]=int(r[field])
    for m in METRICS:
     for suffix in ['low','high','diff']:
      k=m+'_'+suffix;r[k]=float(r[k]);assert math.isfinite(r[k])
     if abs(r[m+'_high']-r[m+'_low']-r[m+'_diff'])>1e-7:issues.append(f'line {line}: {m} diff mismatch')
    if min(r['n_low'],r['n_high'])<=0:issues.append(f'line {line}: empty group')
    if not (0<=r['survival_low']<=1 and 0<=r['survival_high']<=1):issues.append(f'line {line}: survival outside [0,1]')
    condition,seed=r['run'].rsplit('-seed',1)
    if condition not in CONDITIONS or not seed.isdigit():issues.append(f'line {line}: unexpected run {r["run"]}')
    r['condition']=condition;r['seed']=seed;r['weight']=min(r['n_low'],r['n_high'])
    byrun[r['run']].append(r);rows.append(r)
   except (ValueError,AssertionError,KeyError) as e:raise SystemExit(f'Invalid row {line}: {e}')
 seen=set()
 for r in rows:
  k=(r['run'],r['cohort'])
  if k in seen:issues.append('duplicate run/cohort: '+str(k))
  seen.add(k)
 if not rows:raise SystemExit('No matched cohorts')
 runs=[];perseed={};rng=random.Random(72028)
 for run,rr in sorted(byrun.items()):
  w=sum(x['weight'] for x in rr)
  if w<=0:continue
  record={'run':run,'condition':rr[0]['condition'],'seed':rr[0]['seed'],'cohorts':len(rr),'n_low':sum(x['n_low'] for x in rr),'n_high':sum(x['n_high'] for x in rr),'effective_matched_weight':w,'min_cohort_weight':min(x['weight'] for x in rr),'max_cohort_weight':max(x['weight'] for x in rr),'largest_cohort_weight_share':max(x['weight'] for x in rr)/w}
  for m in METRICS:record[m+'_diff']=sum(x['weight']*x[m+'_diff'] for x in rr)/w
  runs.append(record);perseed[(record['condition'],record['seed'])]=record
 fields=['run','condition','seed','cohorts','n_low','n_high','effective_matched_weight','min_cohort_weight','max_cohort_weight','largest_cohort_weight_share']+[m+'_diff' for m in METRICS]
 write(out/'seed_summary.csv',runs,fields)
 cond_rows=[]
 for cond in CONDITIONS:
  rr=[r for r in runs if r['condition']==cond]
  if not rr:issues.append('no matched seeds: '+cond);continue
  rec={'condition':cond,'seeds':len(rr),'cohorts':sum(x['cohorts'] for x in rr),'seeds_with_one_cohort':sum(x['cohorts']==1 for x in rr),'max_cohort_dominance':max(x['largest_cohort_weight_share'] for x in rr)}
  for m in METRICS:
   vals=[x[m+'_diff'] for x in rr];lo,hi=bootstrap(vals,rng,a.bootstrap)
   rec[m+'_mean']=statistics.mean(vals);rec[m+'_sd']=statistics.stdev(vals) if len(vals)>1 else None;rec[m+'_ci_low']=lo;rec[m+'_ci_high']=hi;rec[m+'_positive_seeds']=sum(v>0 for v in vals)
  cond_rows.append(rec)
 cfields=['condition','seeds','cohorts','seeds_with_one_cohort','max_cohort_dominance']+[m+'_'+suffix for m in METRICS for suffix in ['mean','sd','ci_low','ci_high','positive_seeds']]
 write(out/'condition_uncertainty.csv',cond_rows,cfields)
 pair_rows=[]
 for name,c1,c0 in PAIRS:
  seeds=sorted(set(s for c,s in perseed if c==c1)&set(s for c,s in perseed if c==c0))
  for m in ['survival','reproduction']:
   vals=[perseed[(c1,s)][m+'_diff']-perseed[(c0,s)][m+'_diff'] for s in seeds]
   lo,hi=bootstrap(vals,rng,a.bootstrap) if vals else (None,None)
   # paired exact sign-flip randomization (exploratory; exchangeability unverified)
   if vals:
    observed=abs(statistics.mean(vals));n=len(vals)
    exact=sum(abs(sum(v*(1 if (mask>>i)&1 else -1) for i,v in enumerate(vals))/n)>=observed-1e-12 for mask in range(1<<n))/(1<<n)
   else:exact=None
   pair_rows.append({'contrast':name,'metric':m,'paired_seeds':len(vals),'mean_difference_in_resistance_advantage':statistics.mean(vals) if vals else None,'bootstrap_low':lo,'bootstrap_high':hi,'exploratory_signflip_p':exact})
 write(out/'paired_contrasts.csv',pair_rows,list(pair_rows[0]))
 # Context audit: aggregate differences are NOT causal individual-level associations.
 audit=[]
 for cond in CONDITIONS:
  rr=[r for r in runs if r['condition']==cond]
  for exposure in ['toxin_dose','resistance_cost']:
   for outcome in ['survival','reproduction']:
    x=[r[exposure+'_diff'] for r in rr];y=[r[outcome+'_diff'] for r in rr]
    correlation=None
    if len(x)>=3 and len(set(x))>1 and len(set(y))>1:correlation=statistics.correlation(x,y)
    audit.append({'condition':cond,'exposure_difference':exposure,'outcome_difference':outcome,'seed_count':len(rr),'seed_level_correlation':correlation,'interpretation':'ecological association only; exposure is post-birth and survival-dependent'})
 write(out/'confounding_audit.csv',audit,list(audit[0]))
 report=['# Phase 07.2.8 — 選択圧の検出力と交絡要因の検証','',f'- 入力: {source}',f'- SHA256: {hashlib.sha256(source.read_bytes()).hexdigest()}',f'- 比較可能コホート: {len(rows)}','- 独立反復単位: シード（コホート・個体を独立反復としない）',f'- シード単位bootstrap: {a.bootstrap}回、固定乱数seed=72028','- 条件間比較: 共通シードでの耐性差の差（difference in differences）。条件間の対応は同一シード番号のみであり、因果効果を保証しない。','- bootstrap区間は探索的95%パーセンタイル区間。5シードでは不安定。','- 符号反転p値は参考値で、独立・対称性等の仮定は未検証。多重比較補正なし。','', '## 条件別結果（高耐性−低耐性）','','| 条件 | シード | コホート | 生存差 | 95% bootstrap区間 | 繁殖差 | 95% bootstrap区間 | 単一コホートのシード |','|---|---:|---:|---:|---|---:|---|---:|']
 for r in cond_rows:report.append(f'| {r["condition"]} | {r["seeds"]} | {r["cohorts"]} | {fmt(r["survival_mean"])} | [{fmt(r["survival_ci_low"])}, {fmt(r["survival_ci_high"])}] | {fmt(r["reproduction_mean"])} | [{fmt(r["reproduction_ci_low"])}, {fmt(r["reproduction_ci_high"])}] | {r["seeds_with_one_cohort"]} |')
 report+=['','## 交絡と検出力に関する制約','- 毒曝露・維持コストは出生後に生じ、摂食行動や生存時間の影響を受ける。単純な相関を毒の因果効果と解釈しない。','- 元CSVに含まれない個体単位の年齢・生存リスク集合・曝露タイミングの詳細は集計表から復元できない。','- この集計表だけでは正式な検出力・最小検出可能効果（MDE）を厳密に推定できない。標本設計・シード分散・目標効果量・追加シードでの予備実験が必要。','- 高耐性／低耐性は閾値で区分した観測群であり、耐性のランダム割付ではない。','- 5シードでのbootstrap区間やp値は結論の確定に用いない。','- 同一seed番号でも条件ごとに乱数消費順が変わり得るため、対応実験の完全な乱数制御を意味しない。','- 個体別CSVは再処理していない。Phase 07.2.7で生成したコホート表を再利用。','','## 品質検証',f'- 警告: {len(issues)}']
 report += ['- '+x for x in issues] or ['- 追加の形式・整合性警告なし']
 report+=['','## 判定','- 解析・不確実性の可視化: 実施。','- 選択圧の因果的成立: 未確定。','- 次の検証: シード数の増加を検討する前に、観測群の偏り・事前に固定した評価指標・必要な効果量を設計する。']
 (out/'REPORT.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
 (out/'quality.json').write_text(json.dumps({'input_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'cohorts':len(rows),'runs':len(runs),'warnings':issues},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
 print(f'cohorts={len(rows)} runs={len(runs)} warnings={len(issues)} out={out}')
if __name__=='__main__':main()
