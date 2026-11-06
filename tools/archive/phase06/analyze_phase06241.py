#!/usr/bin/env python3
"""Phase 06.2.41: aggregate cumulative predator chase diagnostics per run."""
import csv
from pathlib import Path
import sys
root=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).resolve().parent/'results'/'phase06241'
fields=['p41_visible_samples','p41_approach_samples','p41_retreat_samples','p41_attack_reach_samples','hunt_attempts','hunt_positive_energy','p41_chase_basal_energy','p41_chase_movement_energy','p41_starvation_with_visible_prey','carnivore_energy_deaths']
for mode in ('fixed','ambush','genetic'):
 for path in sorted((root/mode).glob('seed-*.csv')):
  with path.open(encoding='utf-8-sig',newline='') as f:
   rows=list(csv.DictReader(f))
  if not rows:continue
  r=rows[-1]
  if any(k not in r for k in fields):raise ValueError(f'Missing phase41 columns: {path}')
  v={k:float(r[k]) for k in fields}
  seen=v['p41_visible_samples']
  print(f"{mode} {path.stem}: tick={r['tick']} predators={r['carnivores']} visible={int(seen)} approach={int(v['p41_approach_samples'])} retreat={int(v['p41_retreat_samples'])} reach={int(v['p41_attack_reach_samples'])} attacks={int(v['hunt_attempts'])} positive_energy_attacks={int(v['hunt_positive_energy'])} chase_energy={v['p41_chase_basal_energy']+v['p41_chase_movement_energy']:.2f} starving_with_visible_prey={int(v['p41_starvation_with_visible_prey'])} approach_rate={v['p41_approach_samples']/seen if seen else 0:.3f}")
