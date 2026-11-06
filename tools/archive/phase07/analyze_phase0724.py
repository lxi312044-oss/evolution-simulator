#!/usr/bin/env python3
"""Phase 07.2.4: detection limits/confounding audit. Observational; no causal claims."""
import argparse, csv, math, statistics, collections
from pathlib import Path

CONDS=['A_no_toxin_no_cost','B_no_toxin_cost','C_toxin_no_cost','D_toxin_cost','E_strong_no_cost','F_strong_cost']
def read(path):
    with path.open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def num(x,default=0.):
    try:return float(x)
    except (ValueError,TypeError):return default
def save(path,rows,fields):
    path.parent.mkdir(parents=True,exist_ok=True)
    with path.open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(rows)
def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--root',type=Path,default=Path('results/phase0722'))
    p.add_argument('--out',type=Path,default=Path('results/phase0724/analysis'))
    p.add_argument('--window',type=int,default=1000)
    p.add_argument('--cohort-width',type=int,default=1000)
    p.add_argument('--min-bin',type=int,default=5)
    args=p.parse_args()
    if min(args.window,args.cohort_width,args.min_bin)<1:p.error('window/cohort-width/min-bin must be positive')
    runrows=[];cohortrows=[];eventrows=[];problems=[];contrasts=[]
    for cond in CONDS:
        folder=args.root/cond
        for gp in sorted(folder.glob('seed-*.genealogy.csv')):
            seed=gp.name.split('.')[0].split('-')[-1]
            rp=gp.with_name(gp.name.replace('.genealogy.csv','.resistance.csv'))
            ep=gp.with_name(gp.name.replace('.genealogy.csv','.resistance-events.csv'))
            if not rp.exists():problems.append(f'{cond}/{seed}: missing resistance.csv');continue
            g=read(gp);r=read(rp);events=read(ep) if ep.exists() else []
            if not r:problems.append(f'{cond}/{seed}: empty resistance.csv');continue
            end=int(num(r[-1]['tick']))
            people={};children=collections.defaultdict(list);death_events={}
            for e in g:
                key=(e['id_index'],e['id_generation']);kind=e['event'];tick=int(num(e['tick']))
                if kind in ('founder','birth'):
                    if key in people:problems.append(f'{cond}/{seed}: duplicate person {key}');continue
                    people[key]={'born':tick,'res':num(e['child_resistance']),'death':None}
                    if kind=='birth':children[(e['parent_id_index'],e['parent_id_generation'])].append(tick)
                elif kind=='death' and key in people:people[key]['death']=tick
            for e in events:
                if e.get('event')=='death':death_events[(e['id_index'],e['id_generation'])]=e
            missing=sum(k not in people for k in children)
            if missing:problems.append(f'{cond}/{seed}: {missing} unknown parents')
            groups=collections.defaultdict(list)
            for key,v in people.items():
                born=v['born'];res=v['res'];death=v['death']
                if born+args.window>end:continue
                band='low' if res<.49 else ('high' if res>.51 else 'mid')
                survive=int(death is None or death>born+args.window)
                offspring=sum(born<=t<born+args.window for t in children.get(key,[]))
                groups[(born//args.cohort_width,band)].append((key,res,survive,offspring))
            cohortmap=collections.defaultdict(dict)
            for (cohort,band),rows in sorted(groups.items()):
                vals=[x[1] for x in rows];n=len(rows)
                item=dict(condition=cond,seed=seed,cohort_start=cohort*args.cohort_width,band=band,n=n,resistance_sd=statistics.pstdev(vals) if n>1 else 0.,survival_rate=sum(x[2] for x in rows)/n,offspring_per_individual=sum(x[3] for x in rows)/n)
                cohortrows.append(item);cohortmap[cohort][band]=item
            for cohort,bins in cohortmap.items():
                if 'high' not in bins or 'low' not in bins:continue
                hi,lo=bins['high'],bins['low']
                if min(hi['n'],lo['n'])<args.min_bin:continue
                contrasts.append(dict(condition=cond,seed=seed,cohort_start=cohort*args.cohort_width,n_high=hi['n'],n_low=lo['n'],survival_difference=hi['survival_rate']-lo['survival_rate'],offspring_difference=hi['offspring_per_individual']-lo['offspring_per_individual']))
            final=r[-1];init=r[0];active=[x for x in r if num(x['population'])>0]
            initial_var=num(init.get('variance'));max_var=max((num(x.get('variance')) for x in active),default=0)
            final_var=num(final.get('variance')) if num(final['population']) else None
            cost=num(final.get('cumulative_resistance_cost'))
            prevented=num(final.get('cumulative_prevented_damage'))
            damage=num(final.get('cumulative_toxin_damage'))
            exp=num(final.get('cumulative_toxin_exposure'))
            runrows.append(dict(condition=cond,seed=seed,final_population=int(num(final['population'])),founders=sum(v['born']==0 for v in people.values()),eligible_individuals=sum(len(v) for v in groups.values()),matched_cohorts=sum(1 for x in contrasts if x['condition']==cond and x['seed']==seed),initial_variance=initial_var,max_variance=max_var,final_variance='' if final_var is None else final_var,final_mean_resistance=final['mean'] if num(final['population']) else '',cumulative_cost=cost,cumulative_prevented_damage=prevented,cumulative_damage=damage,cumulative_exposure=exp,prevented_per_exposure=prevented/exp if exp>0 else '',cost_per_eligible_individual=cost/max(1,len(people))))
            death_records=[e for e in death_events.values() if e.get('toxin_load','')!='']
            loads=[num(e['toxin_load']) for e in death_records]
            eventrows.append(dict(condition=cond,seed=seed,death_event_count=len(death_records),death_event_mean_toxin_load=statistics.mean(loads) if loads else '',death_event_positive_load_fraction=sum(x>0 for x in loads)/len(loads) if loads else '',note='Death-event toxin load only; NOT individual lifetime exposure, not a causal comparison'))
    if not runrows:raise SystemExit(f'No readable runs under {args.root}; place phase0722 folder at this path')
    save(args.out/'run_detection_audit.csv',runrows,list(runrows[0]))
    save(args.out/'cohort_support.csv',cohortrows,['condition','seed','cohort_start','band','n','resistance_sd','survival_rate','offspring_per_individual'])
    save(args.out/'matched_cohort_effects.csv',contrasts,['condition','seed','cohort_start','n_high','n_low','survival_difference','offspring_difference'])
    save(args.out/'death_event_exposure_proxy.csv',eventrows,list(eventrows[0]))
    # One independent unit per seed. Descriptive 95% t intervals only, not causal estimates or power calculations.
    byseed=collections.defaultdict(list)
    for x in contrasts:byseed[(x['condition'],x['seed'])].append(x)
    seedrows=[]
    for (cond,seed),rows in sorted(byseed.items()):
        seedrows.append(dict(condition=cond,seed=seed,matched_cohorts=len(rows),survival_difference=statistics.mean(x['survival_difference'] for x in rows),offspring_difference=statistics.mean(x['offspring_difference'] for x in rows)))
    save(args.out/'seed_level_effects.csv',seedrows,['condition','seed','matched_cohorts','survival_difference','offspring_difference'])
    report=['# Phase 07.2.4 検出力・交絡要因の診断','',f'- 読み込んだ実験: {len(runrows)} / 30',f'- 比較可能なコホート: {len(contrasts)}',f'- 独立したシード単位の集計: {len(seedrows)}','',
      '## 解析上の制限（重要）','- 既存CSVは個体ごとの生涯毒曝露量・毒ダメージ・維持コストを記録していない。したがって**個体別曝露の交絡調整は不可能**。','- 死亡イベントのtoxin_loadは死亡時点の値だけであり、生涯曝露量の代用にしてはならない。','- 耐性と他遺伝子、空間、餌資源、親系譜の相関は調整していない。','- 5シードでは厳密な検出力（power）を主張できない。個体を独立反復としてp値を算出しない。','- 絶滅シードを除外せず記録し、絶滅時の平均耐性は欠測として扱う。','',
      '## 記述的診断（シードを独立単位とする）','']
    for cond in CONDS:
        rows=[x for x in runrows if x['condition']==cond];ss=[x for x in seedrows if x['condition']==cond]
        vals=[x['offspring_difference'] for x in ss];surv=[x['survival_difference'] for x in ss]
        report.append(f"- **{cond}**: {len(rows)}実験、比較可能シード {len(ss)}、一致コホート {sum(x['matched_cohorts'] for x in ss)}、最大耐性分散の中央値 {statistics.median(x['max_variance'] for x in rows):.8f}、出生差のシード平均 {statistics.mean(vals):+.5f} / シード間SD {statistics.stdev(vals):.5f}" if len(vals)>1 else f'- **{cond}**: 比較可能シード不足')
    report += ['', '## 次の判断', '- 高/低耐性の比較が十分できない条件は検出力不足として記録し、選択がないと断定しない。','- 生涯曝露・維持コスト・餌資源・空間位置・繁殖年齢到達の個体別測定は、必要なら**次フェーズの観測専用変更**として提案する。','- 追加シード数は効果のシード間ばらつきと計測可能性を確認してから決定する。','- 実験の合格基準は変更しない。','']
    if problems:report+=['## 入力データの問題']+['- '+x for x in problems]
    (args.out/'REPORT.md').write_text('\n'.join(report),encoding='utf-8')
    print(f'Wrote {args.out}/REPORT.md; {len(runrows)} runs, {len(contrasts)} matched cohorts; warnings={len(problems)}')
if __name__=='__main__':main()
