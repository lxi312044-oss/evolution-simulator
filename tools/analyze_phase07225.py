#!/usr/bin/env python3
"""Summarize Phase 07.2.25 toxin strength sweep. Standard library only."""
import argparse,csv,math,re,statistics
from pathlib import Path

def rows(path):
    with path.open(encoding='utf-8-sig',newline='') as f:
        yield from csv.DictReader(f)

def num(s):
    try: return float(s)
    except (ValueError,TypeError): return float('nan')

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--root',type=Path,required=True);ap.add_argument('--out',type=Path,required=True);ap.add_argument('--ticks',type=int,default=50000);ap.add_argument('--expected-seeds',type=int,default=20);ap.add_argument('--expected-counts',default='');args=ap.parse_args()
    args.out.mkdir(parents=True,exist_ok=True)
    expected_counts=dict((k,int(v)) for k,v in (part.split('=',1) for part in args.expected_counts.split(',') if part))
    summary=[]; errors=[]
    for path in sorted(args.root.glob('*.resource-reproduction.csv')):
        match=re.search(r'-seed(\d+)-(control|x\d+)\.resource-reproduction\.csv$',path.name)
        if not match: errors.append(f'Unexpected name: {path.name}');continue
        seed,condition=int(match.group(1)),match.group(2)
        life=Path(str(path).replace('.resource-reproduction.csv','.life-events.csv'))
        if not life.exists():errors.append(f'Missing life events: {life.name}');continue
        history=list(rows(path))
        if not history:errors.append(f'Empty: {path.name}');continue
        first,last=history[0],history[-1]
        if int(last['tick'])!=args.ticks:errors.append(f'Incomplete: {path.name} ends {last["tick"]}')
        events=list(rows(life))
        births=[e for e in events if e['event']=='birth']
        deaths=[e for e in events if e['event']=='disappeared']
        censored=[e for e in events if e['event']=='censored']
        if len(censored)!=int(last['herbivores']):errors.append(f'Censored mismatch: {path.name}')
        start_r=num(first['resistance_mean']);end_r=num(last['resistance_mean'])
        initial=int(first['herbivores']);end=int(last['herbivores'])
        # Lifetime births of disappeared individuals and censored survivors; compare as descriptive only.
        fitness=[e for e in events if e['event'] in ('disappeared','censored')]
        low=[num(e['successful_births']) for e in fitness if num(e['resistance'])<0.4]
        high=[num(e['successful_births']) for e in fitness if num(e['resistance'])>=0.6]
        summary.append(dict(condition=condition,seed=seed,initial_herbivores=initial,final_herbivores=end,ever_extinct=int(any(int(x['herbivores'])==0 for x in history)),birth_events=len(births),disappearances=len(deaths),censored=len(censored),initial_resistance_mean=start_r,final_resistance_mean=end_r,resistance_change=end_r-start_r if math.isfinite(end_r) and math.isfinite(start_r) else '',high_mean_offspring=statistics.mean(high) if high else '',low_mean_offspring=statistics.mean(low) if low else '',high_n=len(high),low_n=len(low)))
    if not summary:raise SystemExit('No runs found')
    with (args.out/'run_summary.csv').open('w',newline='',encoding='utf-8') as f:
        w=csv.DictWriter(f,fieldnames=summary[0]);w.writeheader();w.writerows(summary)
    groups=sorted(set(x['condition'] for x in summary),key=lambda x: -1 if x=='control' else int(x[1:]))
    lines=['# Phase 07.2.25 強毒環境の比較','',f'解析ラン: {len(summary)} / 想定 {sum(expected_counts.values()) if expected_counts else args.expected_seeds*len(groups)}','', '|条件|ラン|絶滅ラン|最終個体数中央値|耐性変化中央値|出生数中央値|','|---|---:|---:|---:|---:|---:|']
    for group in groups:
        v=[x for x in summary if x['condition']==group]
        changes=[x['resistance_change'] for x in v if isinstance(x['resistance_change'],float) and math.isfinite(x['resistance_change'])]
        lines.append(f'|{group}|{len(v)}|{sum(x["ever_extinct"] for x in v)}|{statistics.median(x["final_herbivores"] for x in v):.1f}|{statistics.median(changes):+.6f}|' .rstrip('|')+f'{statistics.median(x["birth_events"] for x in v):.1f}|') if changes else lines.append(f'|{group}|{len(v)}|{sum(x["ever_extinct"] for x in v)}|{statistics.median(x["final_herbivores"] for x in v):.1f}|N/A|{statistics.median(x["birth_events"] for x in v):.1f}|')
        expected=expected_counts.get(group,args.expected_seeds)
        if len(v)!=expected:errors.append(f'{group}: expected {expected}, got {len(v)}')
    lines+=['','注: 消失は毒による死亡と同義ではありません。耐性と子孫数の関連は観察的であり、出生時期や曝露の違いを調整していません。','',f'品質上の注意: {len(errors)}件']
    lines += ['- '+e for e in errors]
    (args.out/'REPORT.md').write_text('\n'.join(lines)+'\n',encoding='utf-8')
    (args.out/'quality.txt').write_text('\n'.join(errors) if errors else 'OK\n',encoding='utf-8')
    print('\n'.join(lines))
    if errors:raise SystemExit(1)
if __name__=='__main__':main()
