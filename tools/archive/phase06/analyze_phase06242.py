#!/usr/bin/env python3
"""Phase 42: death vs surviving predator life histories; sidecar is observational."""
import csv,sys
from pathlib import Path
root=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).resolve().parent/'results'/'phase06242'
for mode in ('fixed','ambush','genetic'):
 for path in sorted((root/mode).glob('seed-*.csv.life.csv')):
  with path.open(encoding='utf-8-sig',newline='') as f: rows=list(csv.DictReader(f))
  deaths=[r for r in rows if r['event']=='death']; alive=[r for r in rows if r['event']=='alive_at_end']
  def avg(rs,key):return sum(float(r[key]) for r in rs)/len(rs) if rs else 0
  print(f'{mode} {path.name}: deaths={len(deaths)} survivors={len(alive)} death-visible={sum(int(r["visible_prey_count"])>0 for r in deaths)}')
  for name,rs in [('death',deaths),('survivor',alive)]:
   print(f'  {name}: mean_age={avg(rs,"age"):.1f} mean_attacks={avg(rs,"attacks"):.1f} mean_basal={avg(rs,"basal_spent"):.2f} mean_movement={avg(rs,"movement_spent"):.2f} mean_bite_energy={avg(rs,"bite_energy_received"):.2f} mean_digest={avg(rs,"digestion_received"):.2f} mean_gene={avg(rs,"ambush_gene"):.3f}')
