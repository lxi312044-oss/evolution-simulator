#!/usr/bin/env python3
"""Phase 06.2.43: genetic-only lineage, relative-distance and energy diagnostics."""
import csv
import statistics
import sys
from pathlib import Path

root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent / 'results' / 'phase06243'
paths = sorted((root / 'genetic').glob('seed-*.csv.life.csv'))
if not paths:
    raise SystemExit(f'No genetic life CSVs found under {root / "genetic"}')
for path in paths:
    with path.open(encoding='utf-8-sig', newline='') as f:
        rows = list(csv.DictReader(f))
    deaths = [r for r in rows if r['event'] == 'death']
    alive = [r for r in rows if r['event'] == 'alive_at_end']
    identities = {(r['id_index'], r['id_generation']) for r in rows}
    parent_links = sum(bool(r['parent_id_index']) for r in rows)
    known_parents = sum((r['parent_id_index'], r['parent_id_generation']) in identities for r in rows if r['parent_id_index'])
    def total(field): return sum(int(r[field]) for r in rows)
    def median_rate(group, field):
        values = [float(r[field]) * 1000 / float(r['age']) for r in group if float(r['age']) > 0]
        return statistics.median(values) if values else float('nan')
    print(f'{path.name}: deaths={len(deaths)}, alive={len(alive)}, parent_links={parent_links}, parents_in_life_csv={known_parents}')
    print(f'  relative closer={total("relative_closer")}, farther={total("relative_farther")}, unchanged={total("relative_unchanged")}')
    for label, group in [('death', deaths), ('alive_at_end', alive)]:
        print(f'  {label}: digest/1000tick median={median_rate(group,"digestion_received"):.3f}; basal+movement/1000tick median=', end='')
        costs = [(float(r['basal_spent'])+float(r['movement_spent']))*1000/float(r['age']) for r in group if float(r['age'])>0]
        print(f'{statistics.median(costs):.3f}' if costs else 'n/a')
        print(f'    low-energy-visible individual-ticks={sum(int(r["low_energy_visible_ticks"]) for r in group)}')
