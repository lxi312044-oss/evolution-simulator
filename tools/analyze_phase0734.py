#!/usr/bin/env python3
"""Phase 07.3.4: deterministic long-run stability checks, standard library only.
CSV snapshots cannot prove total nutrient/energy conservation: cargo tests cover ledgers.
"""
import argparse, csv, hashlib, json, math, pathlib, sys

def sha(path):
    h=hashlib.sha256()
    with open(path,'rb') as f:
        for chunk in iter(lambda:f.read(1024*1024),b''): h.update(chunk)
    return h.hexdigest()

def check(path, seed, ticks, interval):
    with open(path,encoding='utf-8-sig',newline='') as f: rows=list(csv.DictReader(f))
    if not rows: raise ValueError('empty CSV')
    required=('seed','tick','herbivores','plants','soil_n','detritus_n','plant_body_n','plant_reserve_n',
              'plant_reserve_e','toxin_dose_cumulative','toxin_damage_cumulative','resistance_low_alive',
              'resistance_mid_alive','resistance_high_alive','resistance_mean','resistance_variance',
              'resistance_min','resistance_max')
    for key in required:
        if key not in rows[0]: raise ValueError('missing column '+key)
    expected_ticks=list(range(0,ticks+1,interval))
    if expected_ticks[-1]!=ticks: expected_ticks.append(ticks)
    observed=[int(r['tick']) for r in rows]
    if observed!=expected_ticks: raise ValueError('tick sequence mismatch')
    for r in rows:
        t=int(r['tick'])
        if int(r['seed'])!=seed: raise ValueError(f'seed mismatch at {t}')
        h=int(r['herbivores']); p=int(r['plants'])
        if min(h,p)<0: raise ValueError(f'negative population at {t}')
        if sum(int(r[k]) for k in ('resistance_low_alive','resistance_mid_alive','resistance_high_alive'))!=h:
            raise ValueError(f'resistance bins mismatch at {t}')
        for k in ('soil_n','detritus_n','plant_body_n','plant_reserve_n','plant_reserve_e',
                  'toxin_dose_cumulative','toxin_damage_cumulative'):
            v=float(r[k])
            if not math.isfinite(v) or v < -1e-6: raise ValueError(f'invalid {k} at {t}: {v}')
        for k in ('resistance_mean','resistance_variance','resistance_min','resistance_max'):
            v=float(r[k])
            if h==0:
                if not math.isnan(v): raise ValueError(f'expected NaN for extinct herbivores {k} at {t}')
            elif not math.isfinite(v): raise ValueError(f'invalid {k} at {t}')
        if h and not (0<=float(r['resistance_min'])<=float(r['resistance_mean'])<=float(r['resistance_max'])<=1.000001):
            raise ValueError(f'resistance bounds invalid at {t}')
    return {'rows':len(rows),'final_tick':observed[-1], 'final_plants':int(rows[-1]['plants']),
            'final_herbivores':int(rows[-1]['herbivores']), 'min_plants':min(int(r['plants']) for r in rows),
            'min_herbivores':min(int(r['herbivores']) for r in rows), 'sha256':sha(path)}

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--first',required=True);ap.add_argument('--second',required=True)
    ap.add_argument('--seed',type=int,default=12345);ap.add_argument('--ticks',type=int,default=30000)
    ap.add_argument('--interval',type=int,default=100);ap.add_argument('--output',required=True)
    a=ap.parse_args(); a.first=pathlib.Path(a.first);a.second=pathlib.Path(a.second)
    if a.ticks<1 or a.interval<1: raise ValueError('ticks/interval must be positive')
    first=check(a.first,a.seed,a.ticks,a.interval);second=check(a.second,a.seed,a.ticks,a.interval)
    if first['sha256']!=second['sha256']: raise ValueError('same-seed long-run CSV SHA256 differs')
    report={'phase':'07.3.4','status':'PASS','seed':a.seed,'ticks':a.ticks,'interval':a.interval,
            'same_seed_reproducible':True,'resource_csv':first,
            'limitations':['This CSV has no full nutrient/energy ledger; cargo conservation tests are required.',
                           'Snapshots cannot prove absence of transient NaN between sampling ticks.',
                           'Carnivore population is not recorded in this CSV.']}
    out=pathlib.Path(a.output);out.parent.mkdir(parents=True,exist_ok=True)
    out.write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
    print('PASS Phase 07.3.4 long-run snapshot and same-seed reproducibility')
    print(json.dumps(report,ensure_ascii=False,indent=2))
if __name__=='__main__':
    try:main()
    except (OSError,ValueError,KeyError) as e:
        print('FAIL Phase 07.3.4:',e,file=sys.stderr);sys.exit(1)
