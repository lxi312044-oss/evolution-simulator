#!/usr/bin/env python3
"""Phase 07.2.2 observational comparisons; no causal claims from raw averages."""
import argparse,csv,math,statistics
from pathlib import Path
CONDS=['A_no_toxin_no_cost','B_no_toxin_cost','C_toxin_no_cost','D_toxin_cost','E_strong_no_cost','F_strong_cost']
def rows(path):
 with path.open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def num(v,default=0.):
 try:return float(v)
 except (TypeError,ValueError):return default
def analyze(root,out,window,cohort_width):
 out.mkdir(parents=True,exist_ok=True)
 summary=[];cohorts=[];extinction=[]
 for condition in CONDS:
  for gp in sorted((root/condition).glob('seed-*.genealogy.csv')):
   seed=gp.name.split('.')[0].split('-')[-1]
   rp=gp.with_name(gp.name.replace('.genealogy.csv','.resistance.csv'))
   if not rp.exists():print('MISSING',rp);continue
   g=rows(gp);r=rows(rp)
   if not r:continue
   end=int(num(r[-1]['tick']));founders={};births={};deaths={};finals={}
   for x in g:
    id=(x['id_index'],x['id_generation']);event=x['event']
    if event=='founder':founders[id]=(0,num(x.get('child_resistance')))
    elif event=='birth':births[id]=(int(num(x['tick'])),num(x['child_resistance']))
    elif event=='death':deaths[id]=int(num(x['tick']))
    elif event=='final':finals[id]=x
    elif x.get('final_status')=='censored':finals[id]=x
   # Parent offspring count from actual birth records (not untrusted summary fields).
   offspring={}
   for x in g:
    if x['event']=='birth':
     parent=(x['parent_id_index'],x['parent_id_generation']);offspring[parent]=offspring.get(parent,0)+1
   individuals={**founders,**births};n_cohorts=0
   buckets={}
   for id,(born,res) in individuals.items():
    if born+window>end:continue # full follow-up required
    cohort=born//cohort_width
    resistance_bin='low' if res<.49 else ('high' if res>.51 else 'mid')
    key=(cohort,resistance_bin)
    died=deaths.get(id)
    survived=(died is None or died>born+window)
    # Total offspring is a lifetime count; cannot attribute it to follow-up window without parent event tick.
    # Calculate window-specific offspring by scanning actual birth events below.
    buckets.setdefault(key,[]).append((id,born,res,survived))
   child_times={}
   for x in g:
    if x['event']=='birth':
     key=(x['parent_id_index'],x['parent_id_generation']);child_times.setdefault(key,[]).append(int(num(x['tick'])))
   for (cohort,b),members in sorted(buckets.items()):
    n=len(members);survivors=sum(v[3] for v in members)
    counts=[sum(born<=t<=born+window for t in child_times.get(id,[])) for id,born,_,_ in members]
    cohorts.append(dict(condition=condition,seed=seed,cohort_start=cohort*cohort_width,resistance_bin=b,n=n,survival_fraction=survivors/n,offspring_per_individual=sum(counts)/n,followup_ticks=window))
   last=r[-1];summary.append(dict(condition=condition,seed=seed,final_tick=end,final_population=last['population'],initial_mean=r[0]['mean'],final_mean=last['mean'] if num(last['population'])>0 else '',final_energy_mean=last['energy_mean'],cumulative_resistance_cost=last['cumulative_resistance_cost'],cumulative_prevented_damage=last['cumulative_prevented_damage'],cumulative_toxin_damage=last['cumulative_toxin_damage'],cumulative_toxin_exposure=last['cumulative_toxin_exposure'],recorded_births=last['cumulative_births'],genealogy_births=sum(x['event']=='birth' for x in g)))
   # Pre-extinction trajectory: last 10 samples before population first reaches zero
   first_zero=next((i for i,x in enumerate(r) if i>0 and num(x['population'])==0),None)
   if first_zero is not None:
    for x in r[max(0,first_zero-10):first_zero+1]:
     extinction.append(dict(condition=condition,seed=seed,extinction_tick=r[first_zero]['tick'],**x))
 def write(name,data):
  if not data:return
  with (out/name).open('w',encoding='utf-8-sig',newline='') as f:
   w=csv.DictWriter(f,fieldnames=list(data[0]));w.writeheader();w.writerows(data)
 write('summary.csv',summary);write('cohort_resistance.csv',cohorts);write('pre_extinction.csv',extinction)
 print(f'Analyzed {len(summary)} runs; {len(cohorts)} cohort groups; {len(extinction)} pre-extinction samples')
 for cond in CONDS:
  group=[x for x in summary if x['condition']==cond]
  if not group:continue
  means=[num(x['final_mean']) for x in group if x['final_mean']!='']
  print(cond,'runs',len(group),'surviving',len(means),'mean final resistance (surviving only)',round(statistics.mean(means),6) if means else 'NA')
 print('CAUTION: cohort bins may contain few animals; offspring and survival comparisons are observational, not controlled for exposure or lineage clustering.')
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--root',type=Path,default=Path('results/phase0722'));ap.add_argument('--out',type=Path,default=Path('results/phase0722/analysis'));ap.add_argument('--window',type=int,default=1000);ap.add_argument('--cohort-width',type=int,default=1000)
 a=ap.parse_args();analyze(a.root,a.out,a.window,a.cohort_width)
