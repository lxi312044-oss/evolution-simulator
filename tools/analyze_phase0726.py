#!/usr/bin/env python3
"""Phase 07.2.6 observational re-evaluation. Standard library only.
Run from project root: python tools/analyze_phase0726.py --root results/phase0725 --out results/phase0726/analysis
"""
import argparse,csv,math,statistics,collections,json
from pathlib import Path

def read(path):
    with path.open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def write(path,rows,fields):
    with path.open('w',encoding='utf-8',newline='') as f:
        w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(rows)
def avg(xs):return statistics.mean(xs) if xs else None
def val(x):return '' if x is None else round(x,7)
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--root',default='results/phase0725');ap.add_argument('--out',default='results/phase0726/analysis');ap.add_argument('--horizon',type=int,default=1000);ap.add_argument('--cohort-width',type=int,default=5000);a=ap.parse_args()
    root=Path(a.root).resolve();out=Path(a.out).resolve();out.mkdir(parents=True,exist_ok=True)
    files=sorted(root.glob('*.individual-life.csv'))
    if not files:raise SystemExit(f'No individual-life CSV in {root}')
    summary=[]; cohorts=[]; quality=[]; runs=[]
    for file in files:
        name=file.name.removesuffix('.individual-life.csv'); life=read(file); genfile=root/(name+'.genealogy.csv')
        if not genfile.exists():quality.append({'run':name,'issue':'missing genealogy'});continue
        gen=read(genfile); births={};parent={}
        for g in gen:
            if g.get('event') in ('birth','founder'):
                key=(g['id_index'],g['id_generation']);births.setdefault(key,int(g['tick']))
                parent[key]=(g.get('parent_id_index',''),g.get('parent_id_generation',''))
        bycohort=collections.defaultdict(lambda:collections.defaultdict(list))
        unmatched=0;ineligible=0
        for r in life:
            key=(r['id_index'],r['id_generation']); born=births.get(key)
            if born is None:unmatched+=1;continue
            resistance=float(r['resistance']); group='low' if resistance<0.49 else 'high' if resistance>0.51 else 'middle'
            # Fixed observation horizon requires full follow-up until death or at least horizon ticks.
            tick=int(r['tick']);age=int(r['age']);end=tick-born
            if born+a.horizon>50000:ineligible+=1;continue
            # Life CSV is only end-of-life; births total and exposure are lifetime metrics,
            # not valid fixed-horizon outcomes for animals surviving past horizon.
            # Death within horizon is observable; surviving beyond horizon is observable.
            survived=int(end>=a.horizon)
            bycohort[born//a.cohort_width][group].append((survived, int(r['successful_births']),float(r['cumulative_toxin_dose']),float(r['cumulative_resistance_cost']),tick,born))
        for c,groups in sorted(bycohort.items()):
            low=groups['low'];high=groups['high']
            if not low or not high:continue
            # survival at fixed horizon is identifiable; lifetime outcomes are only descriptive.
            row={'run':name,'cohort':c,'n_low':len(low),'n_high':len(high),'survival_low':avg([x[0] for x in low]),'survival_high':avg([x[0] for x in high]),'survival_diff':avg([x[0] for x in high])-avg([x[0] for x in low]),'lifetime_births_low':avg([x[1] for x in low]),'lifetime_births_high':avg([x[1] for x in high]),'lifetime_dose_low':avg([x[2] for x in low]),'lifetime_dose_high':avg([x[2] for x in high]),'lifetime_cost_low':avg([x[3] for x in low]),'lifetime_cost_high':avg([x[3] for x in high])}
            cohorts.append(row)
        ds=[r for r in cohorts if r['run']==name];weighted=sum(r['survival_diff']*min(r['n_low'],r['n_high']) for r in ds);weight=sum(min(r['n_low'],r['n_high']) for r in ds)
        summary.append({'run':name,'cohorts':len(ds),'matched_weight':weight,'survival_diff':weighted/weight if weight else None,'missing_birth_records':unmatched,'excluded_late_births':ineligible,'life_rows':len(life)})
        if unmatched:quality.append({'run':name,'issue':f'{unmatched} missing birth records'})
    fields=['run','cohort','n_low','n_high','survival_low','survival_high','survival_diff','lifetime_births_low','lifetime_births_high','lifetime_dose_low','lifetime_dose_high','lifetime_cost_low','lifetime_cost_high']
    write(out/'cohort_comparisons.csv',cohorts,fields)
    write(out/'run_summary.csv',summary,['run','cohorts','matched_weight','survival_diff','missing_birth_records','excluded_late_births','life_rows'])
    write(out/'data_quality.csv',quality,['run','issue'])
    cond=collections.defaultdict(list)
    for r in summary:
        if r['survival_diff'] is not None:cond[r['run'].split('-seed')[0]].append(r['survival_diff'])
    report=['# Phase 07.2.6 — 個体別観測による自然選択の再評価','','## 固定条件','- 解析対象: 既存のPhase 07.2.5 CSV（再シミュレーションなし）',f'- 出生コホート幅: {a.cohort_width} Tick',f'- 生存判定の追跡期間: {a.horizon} Tick','- 耐性区分: low < 0.49 / high > 0.51（middleは対照群から除外）','','## シード別の固定追跡生存差','高耐性−低耐性。コホート内差を小さい群の人数で加重平均。独立した生物個体としての有意差検定は実施しない。','','| 条件 | 有効シード数 | 平均生存差 | シード間SD |','|---|---:|---:|---:|']
    for k,vs in sorted(cond.items()):report.append(f'| {k} | {len(vs)} | {avg(vs):+.5f} | {statistics.stdev(vs):.5f} |' if len(vs)>1 else f'| {k} | {len(vs)} | {avg(vs):+.5f} | — |')
    report+=['','## 解釈上の制限','- 個体別CSVの累積曝露・累積維持コスト・出生数は生涯終了時点の値であり、固定追跡期間内の値ではない。これらを生存・繁殖の因果説明変数として回帰に投入しない。','- 親子の血縁、遺伝的連鎖、位置・資源の差は統制できていない。','- 50,000 Tick以前に絶滅した実験も除外せず、追跡可能な出生コホートを使う。','- 5シードは初期診断であり、シードを独立反復単位として解釈する。','- 現行CSVから固定期間内の出生数は再構成できない。個体ごとの出生イベント時刻が必要。','- 遺伝的浮動と選択を厳密に識別するには追加の独立反復や適切な中立対照が必要。','',f'## 品質確認\n- 入力CSV数: {len(files)}\n- 比較可能なコホート: {len(cohorts)}\n- 警告: {len(quality)}','','## 判定','自然選択の成立は保留。生存差は探索的な指標であり、遺伝性・適応度・世代間変化の3条件をこのレポートだけで満たしたとは判断しない。']
    (out/'REPORT.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
    print('runs',len(summary),'cohorts',len(cohorts),'warnings',len(quality),'out',out)
if __name__=='__main__':main()
