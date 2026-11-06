"""Paired A/B exposure pilot: descriptive only, not a selection-proof test."""
import csv,sys,pathlib,math
root=pathlib.Path(sys.argv[1] if len(sys.argv)>1 else 'results/phase0757')
rows=[]
for seed in (12345,12346):
 for env,expected in [('A80',0.8),('B80',0.2),('equal',0.5)]:
  prefix=f'seed{seed}_{env}'
  with (root/(prefix+'.types.csv')).open(newline='',encoding='utf-8-sig') as f: types=list(csv.DictReader(f))
  with (root/(prefix+'.births.csv')).open(newline='',encoding='utf-8-sig') as f: births=list(csv.DictReader(f))
  with (root/(prefix+'.outcomes.csv')).open(newline='',encoding='utf-8-sig') as f: deaths=list(csv.DictReader(f))
  for ty in ('0','1'):
   samples=[r for r in types if r['toxin_type']==ty]
   first,last=samples[0],samples[-1]
   live=int(last['herbivores'])
   plants0=int(last['toxic_plants'])
   other=int([r for r in types if r['tick']==last['tick'] and r['toxin_type']!=ty][0]['toxic_plants'])
   pairs=[r for r in births if r['toxin_type']==ty]
   death=[r for r in deaths if r['toxin_type']==ty]
   def mean(xs):return sum(xs)/len(xs) if xs else float('nan')
   x=[float(r['parent_resistance']) for r in pairs];y=[float(r['child_resistance']) for r in pairs]
   xm=mean(x);ym=mean(y)
   corr=float('nan')
   if len(x)>2:
    vx=sum((v-xm)**2 for v in x);vy=sum((v-ym)**2 for v in y)
    if vx>0 and vy>0:corr=sum((a-xm)*(b-ym) for a,b in zip(x,y))/math.sqrt(vx*vy)
   rows.append(dict(seed=seed,environment=env,initial_target_type0=expected,toxin_type=ty,initial_n=first['herbivores'],final_n=live,initial_mean=first['mean_resistance'],final_mean=last['mean_resistance'] if live else '',delta_mean=(float(last['mean_resistance'])-float(first['mean_resistance'])) if live else '',final_toxic_plants=plants0,final_toxic_plant_fraction=plants0/(plants0+other) if plants0+other else '',birth_pairs=len(pairs),parent_child_correlation=corr,mean_birth_parent_resistance=xm,mean_child_resistance=ym,death_events=len(death),mean_death_resistance=mean([float(r['resistance']) for r in death]),final_mean_load=last['mean_load'] if live else ''))
out=root/'summary.csv'
with out.open('w',newline='',encoding='utf8') as f:
 w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
print('Wrote',out,len(rows),'rows')
print('Selection cannot be proven by mean change alone. Examine exposure, reproduction and multi-seed robustness.')
