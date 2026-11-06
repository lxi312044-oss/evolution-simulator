from pathlib import Path
import csv,statistics

ROOT=Path(__file__).parent/'results'/'phase06227'
MODES=('streak-100','streak-80','streak-60')
FIELDS=('phase19_plant_births','phase19_plant_age_deaths','phase19_plant_consumed_deaths','phase19_plant_consumed_n','phase19_plant_uptake_n','phase19_plant_photosynthesis_e','phase19_herbivore_births','phase19_herbivore_age_deaths','phase19_herbivore_hp_deaths','phase19_herbivore_feeding_e','phase26_satiety_streak_completions','phase26_satiety_streak_resets')
PERIODS=((0,4000),(4000,8000),(8000,15000),(15000,30000),(30000,50000))
def load(path):
 with path.open(encoding='utf-8-sig',newline='') as f: return {int(r['tick']):r for r in csv.DictReader(f)}
def num(r,k):return float(r.get(k,0) or 0)
def extinct_tick(rows,key):
 seen=False
 for t,r in sorted(rows.items()):
  n=num(r,key)
  if n>0:seen=True
  if seen and n==0:return t
 return None

def main():
 ROOT.mkdir(parents=True,exist_ok=True)
 per=[];windows=[]
 for mode in MODES:
  for f in sorted((ROOT/mode).glob('seed-*.csv')):
   rows=load(f)
   if not rows:continue
   ticks=sorted(rows);end=rows[ticks[-1]]
   p={'mode':mode,'seed':f.stem.removeprefix('seed-'),'last_tick':ticks[-1], 'final_plants':num(end,'plants'),'final_herbivores':num(end,'herbivores'),'final_carnivores':num(end,'carnivores'),'herbivore_peak':max(num(r,'herbivores') for r in rows.values()),'plant_min_after_8000':min((num(r,'plants') for t,r in rows.items() if t>=8000),default=float('nan')),'predator_extinction_tick':extinct_tick(rows,'carnivores'),'herbivore_extinction_tick':extinct_tick(rows,'herbivores'),'max_abs_n_error':max(abs(num(r,'nutrient_error')) for r in rows.values()),'max_abs_e_error':max(abs(num(r,'energy_error')) for r in rows.values())}
   per.append(p)
   for start,stop in PERIODS:
    if start not in rows or stop not in rows:continue
    a,b=rows[start],rows[stop]
    w={'mode':mode,'seed':p['seed'],'start_tick':start,'end_tick':stop,'plants_start':num(a,'plants'),'plants_end':num(b,'plants'),'herbivores_start':num(a,'herbivores'),'herbivores_end':num(b,'herbivores')}
    for k in FIELDS:w[k]=num(b,k)-num(a,k)
    w['plant_births_minus_deaths']=w['phase19_plant_births']-w['phase19_plant_age_deaths']-w['phase19_plant_consumed_deaths']
    w['herbivore_births_minus_deaths']=w['phase19_herbivore_births']-w['phase19_herbivore_age_deaths']-w['phase19_herbivore_hp_deaths']
    windows.append(w)
 def write(name,data):
  if not data:return
  with (ROOT/name).open('w',newline='',encoding='utf-8-sig') as f:
   w=csv.DictWriter(f,fieldnames=list(data[0]));w.writeheader();w.writerows(data)
 write('per_seed_summary.csv',per);write('period_resource_balance.csv',windows)
 print('Phase 06.2.27 summary (actual CSV observations)')
 for mode in MODES:
  x=[r for r in per if r['mode']==mode]
  if not x:print(mode,'NO RESULTS');continue
  avg=lambda k:round(statistics.mean(r[k] for r in x),2)
  print(mode,'runs',len(x),'last ticks',sorted(set(r['last_tick'] for r in x)),'herbivore final mean',avg('final_herbivores'),'survival',sum(r['final_herbivores']>0 for r in x),'plant final mean',avg('final_plants'),'herbivore peak mean',avg('herbivore_peak'),'predator survival',sum(r['final_carnivores']>0 for r in x),'max |N error|',max(r['max_abs_n_error'] for r in x),'max |E error|',max(r['max_abs_e_error'] for r in x))
 print('Saved:',ROOT/'per_seed_summary.csv',ROOT/'period_resource_balance.csv')
 print('Note: phase19 counters are cumulative; per-window values are endpoint differences. Plant deaths include age and consumption; no claim of causal attribution from these counters alone.')
if __name__=='__main__':main()
