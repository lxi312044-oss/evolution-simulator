"""Phase 07.5.6 exploratory selection report. stdlib only. No automatic evolution PASS."""
import csv, pathlib, sys, statistics, math
root=pathlib.Path(sys.argv[1] if len(sys.argv)>1 else 'results/phase0756')
rows=[]
def avg(a): return sum(a)/len(a) if a else float('nan')
def corr(xs,ys):
 if len(xs)<3:return float('nan')
 a=avg(xs);b=avg(ys);v=sum((x-a)**2 for x in xs)*sum((y-b)**2 for y in ys)
 return sum((x-a)*(y-b) for x,y in zip(xs,ys))/math.sqrt(v) if v>0 else float('nan')
for path in sorted(root.glob('*.types.csv')):
 with path.open(newline='',encoding='utf-8-sig') as f: data=list(csv.DictReader(f))
 name=path.name.removesuffix('.types.csv'); birth=root/(name+'.births.csv')
 if not birth.exists():raise RuntimeError('missing births '+str(birth))
 with birth.open(newline='',encoding='utf-8-sig') as f: births=list(csv.DictReader(f))
 for ty in ('0','1'):
  series=[r for r in data if r['toxin_type']==ty]
  if not series:raise RuntimeError('missing type '+ty+' in '+name)
  first,last=series[0],series[-1]
  pairs=[r for r in births if r['toxin_type']==ty]
  x=[float(r['parent_resistance']) for r in pairs];y=[float(r['child_resistance']) for r in pairs]
  # Birth-parent selection relative to population baseline at same observation window is approximate.
  rows.append(dict(run=name,toxin_type=ty,initial_n=first['herbivores'],final_n=last['herbivores'],initial_mean=first['mean_resistance'],final_mean=last['mean_resistance'],delta_mean=float(last['mean_resistance'])-float(first['mean_resistance']),birth_pairs=len(pairs),parent_child_correlation=corr(x,y),mean_parent=avg(x),mean_child=avg(y),final_toxic_plants=last['toxic_plants'],final_mean_load=last['mean_load']))
out=root/'summary.csv'
with out.open('w',newline='',encoding='utf-8') as f:
 w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
print('Wrote',out,'rows',len(rows))
print('CAUTION: two seeds are exploratory; changes in mean resistance do not establish natural selection.')
