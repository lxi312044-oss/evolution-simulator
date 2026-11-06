import argparse,csv,json,math
from pathlib import Path

def analyze(folder,ticks):
    out=[]
    for f in sorted(folder.glob('*-seed*.csv')):
        with f.open(encoding='utf-8-sig',newline='') as h: rows=list(csv.DictReader(h))
        if not rows: raise ValueError(f'empty {f}')
        groups={t:[r for r in rows if r['trait']==t] for t in ('toxic','nontoxic')}
        if any(not g or int(g[-1]['tick'])!=ticks for g in groups.values()): raise ValueError(f'incomplete {f}')
        for trait,group in groups.items():
            prev=None; exposure=0; successes=0; blocked=0; attempts=0
            for r in group:
                for k in ('alive','energy_mean','energy_min','energy_max','seed_attempts','seed_success','seed_resource_blocked'):
                    if not math.isfinite(float(r[k])): raise ValueError(f'nonfinite {f}:{k}')
                if prev is not None:
                    exposure+=(int(prev['alive'])+int(r['alive']))*(int(r['tick'])-int(prev['tick']))/2
                    for k in ('seed_success','seed_resource_blocked','seed_attempts'):
                        if int(r[k])<int(prev[k]): raise ValueError(f'counter regression {f}:{k}')
                prev=r
            successes=int(group[-1]['seed_success']);blocked=int(group[-1]['seed_resource_blocked']);attempts=int(group[-1]['seed_attempts'])
            if successes+blocked>attempts: raise ValueError(f'counter mismatch {f}')
            out.append({'file':f.name,'trait':trait,'seed':int(group[0]['seed']),'final_alive':int(group[-1]['alive']),'final_mean_energy':float(group[-1]['energy_mean']),'final_resource_ready':int(group[-1]['resource_ready']),'parent_seed_success':successes,'seed_attempts':attempts,'resource_blocked':blocked,'plant_tick_exposure_trapezoid':exposure,'parent_success_per_1000_plant_ticks':1000*successes/exposure if exposure else None})
    return {'phase':'07.4.5','status':'PASS','runs':len(out)//2,'traits':out,'limitations':['Sampled energy snapshots cannot reconstruct pre-dispersal energy or per-individual lifetime fitness.','Success counters count successful seed dispersal by parent trait; spontaneous germination is excluded.','Cost changes may alter ecological feedback; same seed does not isolate a per-individual causal effect.']}

def main():
    p=argparse.ArgumentParser();p.add_argument('--input',type=Path,required=True);p.add_argument('--ticks',type=int,required=True);a=p.parse_args()
    report=analyze(a.input,a.ticks)
    (a.input/'REPORT.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
    print(f"PASS Phase 07.4.5: {report['runs']} runs, parent success and energy snapshots")
if __name__=='__main__':main()
