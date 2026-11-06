"""Paired intervention summary. Differences are whole-system effects, not individual causal effects."""
import csv,glob,os,sys
root=sys.argv[1] if len(sys.argv)>1 else 'results/phase0759'
files=sorted(glob.glob(os.path.join(root,'*.types.csv')))
if len(files)!=12:raise SystemExit(f'Expected 12 type CSVs, got {len(files)}')
rows=[]
for file in files:
    run=os.path.basename(file).removesuffix('.types.csv')
    parts=run.split('_')
    seed,environment,protection=parts
    with open(file,encoding='utf-8',newline='') as handle: data=list(csv.DictReader(handle))
    if not data:raise SystemExit(f'Empty {file}')
    for ty in ('0','1'):
        subset=[r for r in data if r['toxin_type']==ty]
        initial,final=subset[0],subset[-1]
        n=int(final['herbivores'])
        rows.append(dict(run=run,seed=seed,environment=environment,protection=protection,toxin_type=ty,
          initial_herbivores=initial['herbivores'],final_herbivores=n,
          initial_resistance=initial['mean_resistance'] if int(initial['herbivores']) else '',
          final_resistance=final['mean_resistance'] if n else '',
          resistance_delta=(float(final['mean_resistance'])-float(initial['mean_resistance'])) if n and int(initial['herbivores']) else '',
          initial_toxic_plants=initial['toxic_plants'],final_toxic_plants=final['toxic_plants'],
          total_births=final['cumulative_herbivore_births'],total_deaths=final['cumulative_herbivore_deaths']))
keys=list(rows[0])
with open(os.path.join(root,'paired_summary.csv'),'w',encoding='utf-8',newline='') as h:
 w=csv.DictWriter(h,fieldnames=keys);w.writeheader();w.writerows(rows)
print('Wrote paired_summary.csv with',len(rows),'type-specific rows')
print('WARNING: effect toggle is resistance_effectiveness (0.8 vs 0.0); upkeep remains 0.5 in both arms.')
print('WARNING: stochastic ecosystem divergence and only two seeds prevent a definitive causal inference about evolution.')
print('WARNING: no survivors means final resistance is missing, not zero.')
