#!/usr/bin/env python3
"""Paired seed pilot: reuse existing Phase 07.2.8 summaries and Phase 07.2.10 cohort comparisons.
No confirmatory significance claims; independent unit = seed.
"""
import argparse, csv, hashlib, json, math, statistics
from pathlib import Path
from statistics import NormalDist
B='B_no_toxin_cost'; F='F_strong_cost'

def read(path,required):
    with path.open(encoding='utf-8-sig',newline='') as fp:
        reader=csv.DictReader(fp)
        missing=set(required)-set(reader.fieldnames or [])
        if missing: raise ValueError(f'{path}: missing columns {sorted(missing)}')
        return list(reader)

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--pilot',default='results/phase0728/analysis/seed_summary.csv')
    p.add_argument('--new',default='results/phase07210/analysis/cohort_comparisons.csv')
    p.add_argument('--out',default='results/phase07210/analysis/design')
    p.add_argument('--target-effect',type=float,default=.02)
    a=p.parse_args()
    if not math.isfinite(a.target_effect) or a.target_effect<=0: p.error('target effect must be positive')
    pilot_path=Path(a.pilot);new_path=Path(a.new);out=Path(a.out);out.mkdir(parents=True,exist_ok=True)
    pilot=read(pilot_path,['condition','seed','survival_diff','reproduction_diff'])
    cohorts=read(new_path,['run','cohort','n_low','n_high','survival_diff','reproduction_diff'])
    entries={}; issues=[]
    for row in pilot:
        if row['condition'] not in (B,F):continue
        key=(row['condition'],int(row['seed']))
        if key in entries:raise ValueError(f'duplicate original summary {key}')
        entries[key]=(float(row['survival_diff']),float(row['reproduction_diff']),'original')
    grouped={}
    for row in cohorts:
        run=row['run']
        if '-seed' not in run:continue
        cond,seed=run.rsplit('-seed',1)
        if cond not in (B,F):continue
        key=(cond,int(seed)); weight=min(int(row['n_low']),int(row['n_high']))
        if weight<=0:raise ValueError(f'invalid cohort weight {run}')
        grouped.setdefault(key,[]).append((weight,float(row['survival_diff']),float(row['reproduction_diff'])))
    for key,rr in grouped.items():
        if key in entries:raise ValueError(f'overlapping original/new seed: {key}')
        w=sum(x[0] for x in rr)
        entries[key]=(sum(x[0]*x[1] for x in rr)/w,sum(x[0]*x[2] for x in rr)/w,'new')
    allseeds=sorted(set(s for c,s in entries))
    paired=[]
    for seed in allseeds:
        if (B,seed) not in entries or (F,seed) not in entries:
            issues.append(f'unpaired seed {seed}');continue
        b=entries[(B,seed)];f=entries[(F,seed)]
        paired.append({'seed':seed,'source':f[2], 'B_survival_diff':b[0],'F_survival_diff':f[0],
                       'F_minus_B_survival':f[0]-b[0], 'F_minus_B_reproduction':f[1]-b[1]})
    with (out/'paired_seed_summary.csv').open('w',encoding='utf-8',newline='') as fp:
        fields=['seed','source','B_survival_diff','F_survival_diff','F_minus_B_survival','F_minus_B_reproduction']
        writer=csv.DictWriter(fp,fieldnames=fields);writer.writeheader();writer.writerows(paired)
    vals=[x['F_minus_B_survival'] for x in paired]
    n=len(vals);sd=statistics.stdev(vals) if n>=2 else None
    mean=statistics.mean(vals) if vals else None
    # Two-sided alpha .05, 80% power, approximate normal calculation (not guaranteed).
    z=NormalDist().inv_cdf(.975)+NormalDist().inv_cdf(.8)
    required=math.ceil((z*sd/a.target_effect)**2) if sd is not None and sd>0 else None
    if n<20:issues.append('fewer than 20 complete paired seeds; variance estimate remains preliminary')
    if sd==0:issues.append('zero observed seed variance; required sample size cannot be estimated')
    if not grouped:issues.append('no newly matched cohorts')
    quality={'paired_seeds':n,'new_paired_seeds':sum(r['source']=='new' for r in paired),
             'original_paired_seeds':sum(r['source']=='original' for r in paired),
             'new_cohort_rows':len(cohorts),'issues':issues,
             'pilot_sha256':hashlib.sha256(pilot_path.read_bytes()).hexdigest(),
             'new_sha256':hashlib.sha256(new_path.read_bytes()).hexdigest()}
    (out/'quality.json').write_text(json.dumps(quality,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    report=['# Phase 07.2.10 — 追加予備シードの分散再推定','',
            '- 主要候補比較: F−B（各条件の高耐性−低耐性生存率差の差）。',
            '- 主要指標: 出生後1,000 Tickの生存率差。副次指標: 同期間の繁殖数差。',
            '- 目標最小効果量: '+str(a.target_effect*100)+' パーセントポイント。',
            '- 反復単位: シード。コホートを独立反復として数えない。',
            f'- 完全ペアシード: {n}（追加シード: {quality["new_paired_seeds"]}）。',
            f'- F−B 平均: {mean:+.6f}' if mean is not None else '- F−B 平均: NA',
            f'- F−B シードSD: {sd:.6f}' if sd is not None else '- F−B シードSD: NA',
            f'- 正規近似による必要シード数概算: {required}' if required is not None else '- 必要シード数: 推定不可',
            '', '## 制約',
            '- 5〜20シードの標準偏差に依存するため、検出力保証ではない。',
            '- 高耐性・低耐性は観測群であり、因果推定ではない。',
            '- 観測コホートがゼロのシードはペア比較から除外される。欠損をゼロ効果と扱わない。',
            '- この再推定は探索的であり、途中で有意性を見て停止する手順ではない。',
            '- 主要比較F−Bは暫定案。正式な確証実験の前に最終承認が必要。',
            '',f'## 品質上の注意: {len(issues)}']
    report += ['- '+x for x in issues] or ['- なし']
    (out/'REPORT.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
    print(f'paired={n}, new={quality["new_paired_seeds"]}, issues={len(issues)}, output={out}')
if __name__=='__main__':main()
