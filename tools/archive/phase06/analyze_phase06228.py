from pathlib import Path
import csv
from collections import defaultdict
root=Path(__file__).resolve().parent/'results/phase06228'
for mode in ('control','body-n-repair','repair-hunting'):
 print('\n',mode)
 for f in sorted((root/mode).glob('seed-*.csv')):
  with f.open(encoding='utf-8-sig',newline='') as h:
   rows=list(csv.DictReader(h))
  if not rows:continue
  last=rows[-1];print(f.stem,'final tick=',last.get('tick'),'carnivores=',last.get('carnivores',last.get('carnivore_count','?')),'herbivores=',last.get('herbivores',last.get('herbivore_count','?')))
