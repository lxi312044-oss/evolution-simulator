#!/usr/bin/env python3
import csv, pathlib, sys
root=pathlib.Path(sys.argv[1]) if len(sys.argv)>1 else pathlib.Path(__file__).resolve().parent/'results'/'phase06239'
for mode in ('fixed','ambush','genetic'):
 for file in sorted((root/mode).glob('seed-*.csv')):
  with file.open(encoding='utf-8-sig',newline='') as f: rows=list(csv.DictReader(f))
  if not rows:continue
  r=rows[-1]; print(f"{mode} {file.stem} tick={r['tick']} predators={r['carnivores']}")
  for i in range(5):
   counts={k:int(r[f'gene_{k}_bin{i}']) for k in ('births','parent_reproductions','deaths','ambush_choices','search_choices','mutations')}
   opportunities=counts['ambush_choices']+counts['search_choices']
   rate=counts['ambush_choices']/opportunities if opportunities else float('nan')
   print(f'  gene bin {i} [{i/5:.1f},{(i+1)/5:.1f}]: {counts} ambush_fraction={rate:.4f}')
