#!/usr/bin/env python3
"""Phase 07.2.27: fixed 5000-tick offspring fitness and inheritance checks. Python stdlib only."""
import argparse, collections, csv, io, math, re, statistics, zipfile
from pathlib import Path

HORIZON=5000
MAX_BIRTH=35000
def mean(x): return statistics.mean(x) if x else None
def med(x): return statistics.median(x) if x else None
def save(path, rows, fields):
    with path.open('w',encoding='utf-8',newline='') as f:
        w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(rows)
def fmt(v):return 'NA' if v is None else f'{v:.6f}'
def analyze(data, condition, seed):
    people={}; kids=collections.defaultdict(list); parent_links=[]; anomalies=[]
    for r in csv.DictReader(io.StringIO(data.decode('utf-8-sig'))):
        try:
            tick=int(r['tick']); ev=r['event']; ident=r['id']
            if ev in ('initial','birth'):
                if ident in people: anomalies.append('duplicate_id')
                people[ident]={'birth':tick,'res':float(r['resistance']),'end':None,'end_event':None}
                if ev=='birth':
                    kids[r['parent_id']].append(tick)
                    parent_links.append((r['parent_id'],ident))
            elif ev in ('disappeared','censored'):
                if ident not in people: anomalies.append('missing_origin')
                else:
                    people[ident]['end']=tick
                    people[ident]['end_event']=ev
        except (ValueError,KeyError): anomalies.append('invalid_row')
    groups={'high':[],'low':[]};cohorts=collections.defaultdict(lambda: {'high':[],'low':[]})
    eligible=0; excluded=0; inherited=[]; offsprings=0
    for ident,p in people.items():
        birth=p['birth'];res=p['res']
        if birth==0 or birth>MAX_BIRTH: continue
        if p['end'] is not None and p['end']<birth: anomalies.append('end_before_birth');continue
        # A disappearance before 5000 is an observed death; offspring after death are zero.
        # A censored endpoint before 5000 is not a complete window.
        if p['end_event']=='censored' and p['end']<birth+HORIZON: excluded+=1;continue
        if p['end'] is None: excluded+=1;continue
        eligible+=1
        count=sum(birth<=t<=birth+HORIZON for t in kids.get(ident,()))
        offsprings+=count
        group='high' if res>=.55 else 'low' if res<=.45 else None
        if group:
            groups[group].append(count)
            cohorts[birth//5000][group].append(count)
    for parent,child in parent_links:
        if parent in people and child in people:
            inherited.append(abs(people[parent]['res']-people[child]['res']))
    return groups,cohorts,dict(eligible=eligible,excluded=excluded,individuals=len(people),offspring=offsprings,inheritance_pairs=len(inherited),mean_parent_child_abs_diff=mean(inherited) if inherited else '',issues=len(anomalies)),anomalies

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--zip',required=True,type=Path)
    ap.add_argument('--out',required=True,type=Path)
    args=ap.parse_args();args.out.mkdir(parents=True,exist_ok=True)
    runs=[];cohort_rows=[];errors=[];files=0
    with zipfile.ZipFile(args.zip) as archive:
        for path in archive.namelist():
            if not path.endswith('.life-events.csv'):continue
            m=re.search(r'-seed(\d+)-(control|x\d+)\.life-events\.csv$',path)
            if not m: errors.append('Unknown file '+path);continue
            files+=1;seed=int(m.group(1));condition=m.group(2)
            groups,cohorts,stats,issues=analyze(archive.read(path),condition,seed)
            errors.extend(f'{path}: {i}' for i in issues)
            hi=mean(groups['high']);lo=mean(groups['low'])
            runs.append(dict(condition=condition,seed=seed,**stats,high_n=len(groups['high']),low_n=len(groups['low']),high_mean_births=hi if hi is not None else '',low_mean_births=lo if lo is not None else '',high_minus_low=(hi-lo) if hi is not None and lo is not None else ''))
            for bin_,v in cohorts.items():
                h=mean(v['high']);l=mean(v['low'])
                cohort_rows.append(dict(condition=condition,seed=seed,birth_tick_start=bin_*5000,birth_tick_end=(bin_+1)*5000,high_n=len(v['high']),low_n=len(v['low']),high_mean=h if h is not None else '',low_mean=l if l is not None else '',high_minus_low=h-l if h is not None and l is not None else ''))
    fields=list(runs[0]) if runs else []
    save(args.out/'fixed_window_runs.csv',runs,fields)
    save(args.out/'fixed_window_cohorts.csv',cohort_rows,list(cohort_rows[0]) if cohort_rows else [])
    summary=[]
    for condition in ['control','x1','x2','x4','x10','x20','x50']:
        rs=[r for r in runs if r['condition']==condition]
        vals=[r['high_minus_low'] for r in rs if isinstance(r['high_minus_low'],float)]
        summary.append(dict(condition=condition,runs=len(rs),comparable=len(vals),median_high_minus_low=med(vals) if vals else '',positive_runs=sum(v>0 for v in vals),total_eligible=sum(r['eligible'] for r in rs),total_excluded=sum(r['excluded'] for r in rs)))
    save(args.out/'summary.csv',summary,list(summary[0]))
    base={r['seed']:r for r in runs if r['condition']=='control'}
    pairs=[]
    for r in runs:
        if r['condition']!='x10' or r['seed'] not in base:continue
        a=r['high_minus_low'];b=base[r['seed']]['high_minus_low']
        pairs.append(dict(seed=r['seed'],control_diff=b,x10_diff=a,x10_minus_control=(a-b) if isinstance(a,float) and isinstance(b,float) else ''))
    save(args.out/'paired_x10_vs_control.csv',pairs,list(pairs[0]) if pairs else ['seed','control_diff','x10_diff','x10_minus_control'])
    x10=next(s for s in summary if s['condition']=='x10')
    paired=[p['x10_minus_control'] for p in pairs if isinstance(p['x10_minus_control'],float)]
    report=f"""# Phase 07.2.27 固定観測期間の繁殖適応度検証

- 既存個体履歴: {files}ラン
- 出生後評価期間: {HORIZON:,} Tick
- 対象出生時刻: 1〜{MAX_BIRTH:,} Tick
- 毒10倍の比較可能ラン: {x10['comparable']}/{x10['runs']}
- 毒10倍の高耐性−低耐性 出生子数差中央値: {fmt(x10['median_high_minus_low'] if isinstance(x10['median_high_minus_low'],float) else None)}
- 毒10倍で正の差を示すラン: {x10['positive_runs']}/{x10['comparable']}
- 毒10倍−毒なしの同seedペア差中央値: {fmt(med(paired))}
- 入力品質警告: {len(errors)}

## 解析上の制約
- 出生時耐性 >=0.55 を高耐性、<=0.45 を低耐性とする。閾値間の個体は比較から除外。
- 5,000 Tick以内に消失した個体は、それ以降の出生子数を0として扱う。
- 観測期間を満たさない打ち切り個体は除外。生存者除外による偏りの可能性に注意。
- 親子間の出生時耐性の絶対差を別途集計。遺伝の正確性を確認できるが、選択圧の因果効果は示さない。
- これは観察的関連であり、因果的な自然選択の確証ではない。
- 各seedの同一初期条件比較は乱数系列の分岐を考慮する。
"""
    (args.out/'REPORT.md').write_text(report,encoding='utf-8')
    (args.out/'quality.txt').write_text('OK\n' if not errors else '\n'.join(errors[:100]),encoding='utf-8')
    print(report)
    if errors:raise SystemExit('Input quality errors; see quality.txt')
if __name__=='__main__':main()
