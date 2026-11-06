#!/usr/bin/env python3
"""Phase 06.2.23: analyze time windows from 100-tick CSV snapshots.
Usage: python analyze_phase06223.py [results/phase06223]
"""
import csv, pathlib, sys
base=pathlib.Path(sys.argv[1] if len(sys.argv)>1 else 'results/phase06223')
windows=[(0,4000),(4000,8000),(8000,15000),(15000,30000),(30000,50000)]
fields=['herbivore_births','herbivore_hp_deaths','herbivore_age_deaths','herbivore_feeding_e','herbivore_absorbed_n','herbivore_digested_n','herbivore_basal_e','herbivore_movement_e','phase21_starvation_hp_damage','phase20_repro_energy_gate_blocked','phase20_repro_satiety_blocked','phase20_repro_body_n_blocked','phase20_repro_reserve_n_blocked','phase20_feeding_in_range_success_ticks']
output=[]
for file in sorted(base.glob('*/seed-*.csv')):
 with file.open(encoding='utf-8-sig',newline='') as f: rows=list(csv.DictReader(f))
 if not rows: continue
 by_tick={int(float(r['tick'])):r for r in rows}
 for start,end in windows:
  if start not in by_tick or end not in by_tick: continue
  a,b=by_tick[start],by_tick[end]
  out={'mode':file.parent.name,'seed':file.stem.removeprefix('seed-'),'from_tick':start,'to_tick':end,
       'plants_start':a.get('plants',''),'plants_end':b.get('plants',''),
       'herbivores_start':a.get('herbivores',''),'herbivores_end':b.get('herbivores',''),
       'carnivores_end':b.get('carnivores','')}
  for key in fields:
   if key in a and key in b:
    out[key+'_delta']=float(b[key])-float(a[key])
  for key in ['nutrient_error','energy_error']:
   if key in b: out[key]=b[key]
  output.append(out)
if not output:
 raise SystemExit('No CSV data found at '+str(base)+' (check paths and tick snapshots)')
path=base/'phase06223_period_analysis.csv'
with path.open('w',newline='',encoding='utf-8-sig') as f:
 writer=csv.DictWriter(f,fieldnames=list(dict.fromkeys(k for r in output for k in r)))
 writer.writeheader();writer.writerows(output)
print('Wrote',path,'rows=',len(output))
