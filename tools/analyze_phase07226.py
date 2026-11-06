#!/usr/bin/env python3
"""Phase 07.2.26: paired run-level selection and lifetime fitness diagnostics. stdlib only."""
import argparse,csv,io,zipfile,statistics,math,collections,re
from pathlib import Path

def f(x):
    try:
        v=float(x);return v if math.isfinite(v) else None
    except (TypeError,ValueError):return None

def write(path,rows,fields):
    with path.open('w',encoding='utf-8',newline='') as h:
        w=csv.DictWriter(h,fieldnames=fields);w.writeheader();w.writerows(rows)

def median(v):return statistics.median(v) if v else None

def main():
    p=argparse.ArgumentParser();p.add_argument('--zip',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(a.zip) as z:
        summary_name=next((n for n in z.namelist() if n.endswith('/analysis/run_summary.csv')),None)
        if not summary_name:raise SystemExit('Missing run_summary.csv')
        summary=list(csv.DictReader(io.StringIO(z.read(summary_name).decode('utf-8-sig'))))
        runs=[]; cohort=[]; errors=[]
        names=[n for n in z.namelist() if n.endswith('.life-events.csv')]
        for n in names:
            m=re.search(r'-seed(\d+)-(control|x\d+)\.life-events\.csv$',n)
            if not m:errors.append('Unrecognized: '+n);continue
            seed=int(m.group(1));condition=m.group(2)
            people={};births=collections.Counter();birth_pairs=[]
            try:
                reader=csv.DictReader(io.StringIO(z.read(n).decode('utf-8-sig')))
                for r in reader:
                    ev=r['event'];id=r['id'];tick=int(r['tick']);res=f(r['resistance'])
                    if ev in ('initial','birth'):
                        if id in people:errors.append('Duplicate birth '+n+' '+id)
                        people[id]=(tick,res)
                        if ev=='birth':
                            parent=r['parent_id'];births[parent]+=1
                            birth_pairs.append((parent,id,res))
                    elif ev in ('disappeared','censored'):
                        if id not in people:errors.append('Missing origin '+n+' '+id)
            except (KeyError,ValueError) as ex:errors.append('Malformed '+n+': '+str(ex));continue
            paired=[];bins=collections.defaultdict(list)
            for parent,child,child_res in birth_pairs:
                if parent in people and child_res is not None and people[parent][1] is not None:
                    paired.append(abs(child_res-people[parent][1]))
            for id,(tick,res) in people.items():
                if res is None:continue
                # Early-born cohorts, 0-10k, excluding late-born short follow-up.
                if tick>=10000 or tick==0:continue
                # fixed bin comparison, no post-treatment adjustment
                group='high' if res>=0.55 else 'low' if res<=0.45 else 'middle'
                bins[group].append(births[id])
            high=bins['high'];low=bins['low']
            record=dict(condition=condition,seed=seed,individuals=len(people),birth_events=len(birth_pairs),parent_child_pairs=len(paired),mean_abs_parent_child_delta=statistics.mean(paired) if paired else '',early_high_n=len(high),early_low_n=len(low),early_high_mean_births=statistics.mean(high) if high else '',early_low_mean_births=statistics.mean(low) if low else '',early_high_minus_low=(statistics.mean(high)-statistics.mean(low)) if high and low else '')
            runs.append(record)
        by={(r['condition'],int(r['seed'])):r for r in summary}
        paired=[]
        for seed in sorted({int(r['seed']) for r in summary}):
            base=by.get(('control',seed));x10=by.get(('x10',seed))
            if not base or not x10:continue
            a1=f(x10['resistance_change']);b1=f(base['resistance_change'])
            paired.append(dict(seed=seed,control_change=b1,x10_change=a1,x10_minus_control=(a1-b1) if a1 is not None and b1 is not None else '',control_births=int(base['birth_events']),x10_births=int(x10['birth_events']),x10_minus_control_births=int(x10['birth_events'])-int(base['birth_events'])))
        write(a.out/'individual_fitness_runs.csv',runs,list(runs[0]) if runs else [])
        write(a.out/'paired_x10_vs_control.csv',paired,list(paired[0]) if paired else [])
        delta=[r['x10_minus_control'] for r in paired if isinstance(r['x10_minus_control'],float)]
        x10=[r for r in runs if r['condition']=='x10'];available=[r['early_high_minus_low'] for r in x10 if isinstance(r['early_high_minus_low'],float)]
        report=['# Phase 07.2.26 毒10倍前後の進化成立条件の検証','',f'- 個体履歴ファイル: {len(runs)}','- 比較対象: 既存140ラン（新規シミュレーションなし）',f'- 同一seedで比較可能なx10対control: {len(paired)}',f'- x10−control 耐性変化差の中央値: {median(delta)}',f'- x10の早期出生個体で高耐性−低耐性の生涯出生子数差（比較可能ラン）: {len(available)}/{len(x10)}',f'- 同差の中央値: {median(available)}','', '## 解釈','- 耐性上昇と選択による進化は同義ではない。比較可能な初期出生コホートの高低耐性差と親子伝達を合わせて検討する。','- 高低群は出生時耐性>=0.55 / <=0.45。両群が揃わないランは欠測とし、ゼロに置き換えない。','- 同じseedでも乱数系列が途中から分岐するため、ペア差は完全な因果効果ではない。','- 生涯出生子数には死亡・打ち切りの観測期間差が残る。死亡原因は直接特定できない。','- 毒8/12/15倍は未実行。追加実験を行う場合はまず各2seedの予備検証を推奨。', '',f'## 品質警告 ({len(errors)})']+[f'- {e}' for e in errors[:30]]
        (a.out/'REPORT.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
        (a.out/'quality.txt').write_text('\n'.join(errors) if errors else 'OK\n',encoding='utf-8')
        print('\n'.join(report[:10]));
        if errors:raise SystemExit(1)
if __name__=='__main__':main()
