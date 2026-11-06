#!/usr/bin/env python3
"""Phase 07.2.23: observational within-run fitness associations; stdlib only.
Never treat missing terminal events as deaths. Never interpret post-outcome dose as randomized exposure.
"""
import argparse,csv,math,random,statistics,json
from collections import defaultdict,Counter
from pathlib import Path

def write(path,fields,rows):
 with open(path,'w',encoding='utf-8',newline='') as f:
  w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(rows)
def solve(a,b):
 n=len(b);a=[list(map(float,r))+[float(b[i])] for i,r in enumerate(a)]
 for k in range(n):
  j=max(range(k,n),key=lambda i:abs(a[i][k]));a[k],a[j]=a[j],a[k]
  if abs(a[k][k])<1e-10:return None
  t=a[k][k];a[k]=[v/t for v in a[k]]
  for i in range(n):
   if i==k:continue
   t=a[i][k];a[i]=[v-t*w for v,w in zip(a[i],a[k])]
 return [r[-1] for r in a]
def ols(data,cols):
 # intercept, resistance per +0.1, birth tick per 10k, optional cumulative toxin dose per 10
 k=len(cols);xx=[[0.]*k for _ in range(k)];xy=[0.]*k
 for r in data:
  x=[r[c] for c in cols];y=r['children']
  for i in range(k):
   xy[i]+=x[i]*y
   for j in range(k):xx[i][j]+=x[i]*x[j]
 return solve(xx,xy)
def ci(vals):
 if not vals:return (None,None,None)
 m=statistics.mean(vals);rng=random.Random(723);b=[]
 for _ in range(1200):b.append(statistics.mean(rng.choices(vals,k=len(vals))))
 b.sort();return (m,b[30],b[1169])
def main():
 p=argparse.ArgumentParser();p.add_argument('--input',default='results/phase072221/analysis/individual_fitness.csv');p.add_argument('--out',default='results/phase07223/analysis');p.add_argument('--ticks',type=int,default=50000);a=p.parse_args()
 out=Path(a.out);out.mkdir(parents=True,exist_ok=True)
 runs=defaultdict(list);errors=[];total=0
 with open(a.input,encoding='utf-8-sig',newline='') as f:
  for i,d in enumerate(csv.DictReader(f),2):
   try:
    if d['origin']!='birth':continue # Initial individuals have unknown birth dates / left truncation
    r={'res':float(d['resistance_at_origin']),'children':int(d['observed_children']),'birth':int(d['origin_tick']), 'status':d['status'],'dose':float(d['toxin_dose_at_terminal']) if d['toxin_dose_at_terminal'] else None}
    if not (0<=r['res']<=1 and 0<=r['birth']<=a.ticks and r['children']>=0 and math.isfinite(r['res'])):raise ValueError('invalid values')
    r.update(intercept=1.,res01=(r['res']-0.5)/0.1,birth10k=r['birth']/10000.)
    runs[(d['level'],int(d['seed']))].append(r);total+=1
   except (ValueError,KeyError) as e:errors.append(f'line {i}: {e}')
 if errors:raise ValueError('Input validation failed: '+str(errors[:5]))
 details=[];by_level=defaultdict(list);cohorts=defaultdict(lambda:[0,0,0.,0.]);censored=Counter()
 for (level,seed),data in sorted(runs.items()):
  completed=[r for r in data if r['status']=='observed_disappearance'];censored[(level,seed)]=len(data)-len(completed)
  fit=ols(data,['intercept','res01','birth10k']) if len(data)>10 else None
  fit_completed=ols(completed,['intercept','res01','birth10k']) if len(completed)>10 else None
  dose_data=[dict(r,dose10=r['dose']/10.) for r in completed if r['dose'] is not None]
  fit_dose=ols(dose_data,['intercept','res01','birth10k','dose10']) if len(dose_data)>10 else None
  row={'level':level,'seed':seed,'born':len(data),'completed':len(completed),'unobserved_terminal':len(data)-len(completed),'resistance_std':statistics.pstdev(r['res'] for r in data),'mean_children':statistics.mean(r['children'] for r in data),'slope_children_per_0p1_resistance':fit[1] if fit else '', 'completed_only_slope':fit_completed[1] if fit_completed else '', 'completed_dose_adjusted_slope':fit_dose[1] if fit_dose else ''}
  details.append(row)
  if fit:by_level[level].append((seed,fit[1]))
  for r in data:
   # 5000 tick cohort + absolute resistance bins. All censoring retained, never labeled death.
   band='low' if r['res']<0.45 else ('high' if r['res']>0.55 else 'mid')
   key=(level,r['birth']//5000,band);cohorts[key][0]+=1;cohorts[key][1]+=r['children'];cohorts[key][2]+=r['res'];cohorts[key][3]+=r['status']!='observed_disappearance'
 summary=[]
 for level in ['control','x1','x2','x4']:
  pairs=by_level[level];values=[v for _,v in pairs];m,lo,hi=ci(values)
  summary.append({'level':level,'runs':len(values),'mean_slope_per_0p1_resistance':m,'bootstrap_95_low':lo,'bootstrap_95_high':hi,'positive_runs':sum(v>0 for v in values),'negative_runs':sum(v<0 for v in values),'interpretation':'association only; includes terminally unobserved births'})
 paired=[];base=dict(by_level['control'])
 for level in ['x1','x2','x4']:
  delta=[(seed,v-base[seed]) for seed,v in by_level[level] if seed in base]
  m,lo,hi=ci([v for _,v in delta]);paired.append({'level':level,'paired_seeds':len(delta),'mean_slope_difference_vs_control':m,'bootstrap_95_low':lo,'bootstrap_95_high':hi,'positive_pairs':sum(v>0 for _,v in delta)})
 co=[]
 for (level,cohort,band),(n,children,res,cens) in sorted(cohorts.items()):co.append({'level':level,'birth_tick_from':cohort*5000,'birth_tick_to_exclusive':(cohort+1)*5000,'resistance_band':band,'individuals':n,'mean_resistance':res/n,'mean_observed_children':children/n,'terminally_unobserved':int(cens)})
 write(out/'run_slopes.csv',list(details[0]),details);write(out/'selection_summary.csv',list(summary[0]),summary);write(out/'paired_vs_control.csv',list(paired[0]),paired);write(out/'birth_cohorts.csv',list(co[0]),co)
 issues=[]
 if len(runs)!=80:issues.append(f'Expected 80 runs, got {len(runs)}')
 if any(len(by_level[x])!=20 for x in ['control','x1','x2','x4']):issues.append('Missing run-level estimates')
 if any(x['resistance_std']<1e-8 for x in details):issues.append('Some runs have zero resistance variation')
 (out/'quality.json').write_text(json.dumps({'runs':len(runs),'birth_cohort_individuals':total,'terminally_unobserved':sum(censored.values()),'issues':issues,'limitations':['Birth cohort only: initial animals excluded','Terminally unobserved individuals are not counted as deaths','Observed offspring are right-censored for late-born individuals','Dose measured at terminal is post-baseline and potentially outcome-dependent','OLS association is not a causal selection estimate','Run-level bootstrap is not a mechanistic test of heredity','Resistance mutation may create low/high band imbalance']},indent=2,ensure_ascii=False),encoding='utf-8')
 lines=['# Phase 07.2.23 — resistance and reproductive fitness associations','',f'Birth-cohort individuals: {total:,}; runs: {len(runs)}; terminally unobserved: {sum(censored.values()):,}.','', 'Slope is the change in observed offspring count per +0.1 resistance, adjusted for birth tick **within each run**. Bootstrap confidence intervals resample 20 runs (not individual animals).','', '|Condition|Mean slope|95% bootstrap interval|Positive runs|','|---|---:|---:|---:|']
 for r in summary:lines.append(f"|{r['level']}|{r['mean_slope_per_0p1_resistance']:.4f}|[{r['bootstrap_95_low']:.4f}, {r['bootstrap_95_high']:.4f}]|{r['positive_runs']}/{r['runs']}|")
 lines+=['','## Matched-seed differences vs no toxin','', '|Condition|Difference|95% bootstrap interval|','|---|---:|---:|']
 for r in paired:lines.append(f"|{r['level']}|{r['mean_slope_difference_vs_control']:.4f}|[{r['bootstrap_95_low']:.4f}, {r['bootstrap_95_high']:.4f}]|")
 lines+=['','## Critical limitations','', '- This is observational association, not proof of natural selection or causal toxin effects.','- Birth tick adjustment addresses only one confounder; environmental resources, lineage, and survivorship may remain confounded.','- Individuals without terminal records are **not** classified as deaths; late births have less time to reproduce.','- Cumulative toxin dose is measured at terminal and is itself affected by lifetime and feeding; `completed_dose_adjusted_slope` is sensitivity analysis, not an unbiased toxin effect.','- The initial animals are excluded because their birth dates and previous reproduction are unknown.','- Selection across generations requires explicit parent-offspring trait transmission and allele/trait frequency trajectories; this dataset alone cannot prove it.','',f"Quality issues: {len(issues)}",'']
 (out/'REPORT.md').write_text('\n'.join(lines),encoding='utf-8')
 print('\n'.join(lines[:20]));print('Quality issues:',issues)
if __name__=='__main__':main()
