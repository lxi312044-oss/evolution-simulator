#!/usr/bin/env python3
"""Phase 06.2.37 matched-seed summary. Standard-library only."""
import csv, pathlib, statistics, sys
root=pathlib.Path(__file__).resolve().parent/'results'/'phase06237'
if len(sys.argv)>1: root=pathlib.Path(sys.argv[1])
for mode in ('fixed','ambush','genetic'):
    runs=[]
    for path in sorted((root/mode).glob('seed-*.csv')):
        with path.open(encoding='utf-8-sig',newline='') as f: rows=list(csv.DictReader(f))
        if not rows: continue
        final=rows[-1]
        alive=[r for r in rows if int(r['carnivores'])>0]
        def num(r,key): return float(r[key])
        max_n=max(abs(num(r,'nutrient_error')) for r in rows)
        max_e=max(abs(num(r,'energy_error')) for r in rows)
        runs.append(dict(seed=path.stem,final_tick=int(final['tick']),herbivores=int(final['herbivores']),carnivores=int(final['carnivores']),last_carnivore_tick=int(alive[-1]['tick']) if alive else -1, births=int(final['carnivore_births']),hunts=int(final['carnivore_hunts']),gene_first=num(alive[0],'carnivore_ambush_gene_mean') if alive else None,gene_last=num(alive[-1],'carnivore_ambush_gene_mean') if alive else None, gene_var_last=num(alive[-1],'carnivore_ambush_gene_variance') if alive else None,max_n_error=max_n,max_e_error=max_e))
    print('\n'+mode+' runs='+str(len(runs)))
    for r in runs: print(', '.join(f'{k}={v}' for k,v in r.items()))
    if runs:
        avg=lambda key:statistics.mean(r[key] for r in runs)
        print(f"carnivore_survival={sum(r['carnivores']>0 for r in runs)}/{len(runs)}, herbivore_survival={sum(r['herbivores']>0 for r in runs)}/{len(runs)}, mean_last_carnivore_tick={avg('last_carnivore_tick'):.1f}, mean_hunts={avg('hunts'):.1f}, max_abs_N={max(r['max_n_error'] for r in runs):.9g}, max_abs_E={max(r['max_e_error'] for r in runs):.9g}")
