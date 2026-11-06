from pathlib import Path
import csv
root=Path(__file__).parent/'results'/'phase06226'
for mode in ('control','satiety-045','satiety-040','streak-60'):
    print('\n',mode)
    for f in sorted((root/mode).glob('seed-*.csv')):
        with f.open(newline='',encoding='utf-8-sig') as fh: rows=list(csv.DictReader(fh))
        if not rows: continue
        last=rows[-1]
        at=lambda tick: next((r for r in rows if int(r['tick'])==tick),None)
        early=at(8000)
        births=lambda r:int(r['phase19_herbivore_births'])
        print(f.stem,'end',last['herbivores'],'plants',last['plants'],'pred',last['carnivores'], 'births_after_8k',births(last)-births(early) if early else 'n/a','streaks',last['phase26_satiety_streak_completions'],'resets',last['phase26_satiety_streak_resets'],'N_err',last['nutrient_error'],'E_err',last['energy_error'])
