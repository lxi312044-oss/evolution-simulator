#!/usr/bin/env python3
"""Phase 07.1: paired seed comparison and baseline reproducibility checks."""
import argparse,csv,json,math
from pathlib import Path
from zipfile import ZipFile

FIELDS=('plants','herbivores','carnivores','carnivore_births','carnivore_deaths','nutrient_error','energy_error','phase19_herbivore_hp_deaths','phase19_herbivore_age_deaths')

def load_csv(path):
    with path.open(encoding='utf-8-sig',newline='') as f: return list(csv.DictReader(f))

def baseline_rows(zip_path,seed):
    with ZipFile(zip_path) as z:
        files=[n for n in z.namelist() if n.endswith(f'/seed-{seed}.csv')]
        if len(files)!=1: raise ValueError(f'baseline seed {seed}: expected one CSV, found {len(files)}')
        return list(csv.DictReader(z.read(files[0]).decode('utf-8-sig').splitlines()))

def compare(a,b,fields):
    if len(a)!=len(b): return [f'row count {len(a)} vs {len(b)}']
    diffs=[]
    for i,(x,y) in enumerate(zip(a,b)):
        if x.get('tick')!=y.get('tick'): diffs.append(f'row {i} tick mismatch');continue
        for k in fields:
            if k not in x or k not in y: raise KeyError(f'missing field {k}')
            try:
                if not math.isclose(float(x[k]),float(y[k]),rel_tol=0,abs_tol=1e-6):
                    diffs.append(f'tick {x["tick"]} {k}: {x[k]} vs {y[k]}')
            except ValueError: diffs.append(f'non-numeric {k}')
            if len(diffs)>=12:return diffs
    return diffs

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--results',default='results/phase071');ap.add_argument('--baseline',default='baselines/phase06243/phase06243-results.zip');ap.add_argument('--ticks',type=int,default=50000);ap.add_argument('--seed-start',type=int,default=12345);ap.add_argument('--seed-count',type=int,default=5)
    args=ap.parse_args();root=Path(__file__).resolve().parent;out=[];problems=[]
    for seed in range(args.seed_start,args.seed_start+args.seed_count):
        rows={}
        for mode in ('off','on'):
            path=root/args.results/mode/f'seed-{seed}.csv'
            if not path.exists():problems.append(f'missing {path}');continue
            rows[mode]=load_csv(path)
            if not rows[mode] or int(rows[mode][-1]['tick'])!=args.ticks:problems.append(f'incomplete {mode}/{seed}')
        if len(rows)!=2:continue
        base=baseline_rows(root/args.baseline,seed)
        diff=compare(base,rows['off'],FIELDS)
        if diff:problems.append(f'baseline/off seed {seed}: '+ '; '.join(diff))
        a,b=rows['off'][-1],rows['on'][-1]
        row={'seed':seed,'off_matches_baseline':not diff,'off':{k:a[k] for k in FIELDS},'on':{k:b[k] for k in FIELDS},'delta':{k:round(float(b[k])-float(a[k]),9) for k in FIELDS}}
        out.append(row)
        print(f'seed {seed}: baseline/off={"MATCH" if not diff else "MISMATCH"}, herbivores off/on={a["herbivores"]}/{b["herbivores"]}, carnivores off/on={a["carnivores"]}/{b["carnivores"]}, energy_error off/on={a["energy_error"]}/{b["energy_error"]}')
    dest=root/args.results/'comparison.json';dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(json.dumps({'runs':out,'problems':problems},ensure_ascii=False,indent=2),encoding='utf-8')
    print('Comparison:',dest)
    if problems:
        print('FAIL:',*problems,sep='\n - ')
        raise SystemExit(1)
    print('PASS: all off runs match baseline on selected metrics; paired results complete')
if __name__=='__main__':main()
