import argparse,csv,json
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--root',default='results/phase0721');a=p.parse_args()
root=Path(a.root);report={}
for condition in ('off','toxin_no_resistance','toxin_resistance'):
 entries=[]
 for f in sorted((root/condition).glob('seed-*.resistance.csv')):
  with f.open(encoding='utf-8-sig',newline='') as h: rows=list(csv.DictReader(h))
  if not rows: continue
  first,last=rows[0],rows[-1]
  if sum(int(last[f'bin{i}']) for i in range(5))!=int(last['population']):raise ValueError(f'bin count mismatch: {f}')
  entries.append({'seed':f.name.split('-')[1].split('.')[0],'ticks':int(last['tick']),'population':int(last['population']),'initial_mean':float(first['mean']),'final_mean':float(last['mean']),'final_variance':float(last['variance']),'births':int(last['cumulative_births']),'deaths':int(last['cumulative_deaths']),'mutations':int(last['cumulative_mutations']),'maintenance_cost':float(last['cumulative_resistance_cost']),'prevented_damage':float(last['cumulative_prevented_damage']),'toxin_damage':float(last['cumulative_toxin_damage'])})
 report[condition]=entries
 print(condition, 'runs=',len(entries))
 for e in entries:print(' ',e)
(root/'evolution_report.json').write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print('Report:',root/'evolution_report.json')
