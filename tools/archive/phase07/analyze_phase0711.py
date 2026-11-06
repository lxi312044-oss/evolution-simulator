#!/usr/bin/env python3
"""Report cumulative toxin mechanisms from Phase 07.1.1 CSVs, no causal overclaim."""
import argparse,csv,json
from pathlib import Path
METRICS=['toxin_exposure_events','toxin_exposure_total','toxin_clearance_total','toxin_hp_damage_total','toxin_damage_ticks','toxin_plant_energy_cost_total','toxin_deaths_with_burden','toxin_deaths_with_burden_and_hp_zero']
def main():
 p=argparse.ArgumentParser();p.add_argument('--results',default='results/phase071');p.add_argument('--seed-start',type=int,default=12345);p.add_argument('--seed-count',type=int,default=5);p.add_argument('--ticks',type=int,default=50000);a=p.parse_args()
 root=Path(__file__).resolve().parent; report=[]
 for seed in range(a.seed_start,a.seed_start+a.seed_count):
  pair={}
  for mode in ('off','on'):
   path=root/a.results/mode/f'seed-{seed}.csv'
   with path.open(encoding='utf-8-sig',newline='') as f:rows=list(csv.DictReader(f))
   if not rows or int(rows[-1]['tick'])!=a.ticks:raise ValueError(f'incomplete: {path}')
   missing=[k for k in METRICS if k not in rows[-1]]
   if missing:raise ValueError(f'missing diagnostic columns in {path}: {missing}')
   d={k:float(rows[-1][k]) for k in METRICS}
   if any(d[k]<0 for k in METRICS):raise ValueError(f'negative counter: {path}')
   if mode=='off' and any(d.values()):raise ValueError(f'toxin OFF has nonzero counters: {path}')
   if any(float(row['toxin_exposure_total'])>d['toxin_exposure_total']+1e-8 for row in rows):raise ValueError(f'nonmonotone cumulative exposure: {path}')
   pair[mode]=d
  report.append({'seed':seed,**pair})
  print(f"seed {seed}: dose={pair['on']['toxin_exposure_total']:.3f}, cleared={pair['on']['toxin_clearance_total']:.3f}, hp_damage={pair['on']['toxin_hp_damage_total']:.3f}, production_energy={pair['on']['toxin_plant_energy_cost_total']:.3f}, deaths_with_burden={int(pair['on']['toxin_deaths_with_burden'])}")
 out=root/a.results/'toxin_diagnostics.json';out.write_text(json.dumps({'runs':report,'note':'deaths_with_burden is association, NOT toxin-attributed deaths'},indent=2),encoding='utf-8');print(out)
if __name__=='__main__':main()
