#!/usr/bin/env python3
"""Phase 07.2.28: pedigree, generation-depth and selection decomposition; stdlib only."""
import argparse,csv,io,zipfile,re,statistics,collections,math
from pathlib import Path

def mean(xs): return statistics.fmean(xs) if xs else None
def med(xs): return statistics.median(xs) if xs else None
def out(path,rows,columns):
    with path.open('w',encoding='utf-8',newline='') as f:
        w=csv.DictWriter(f,fieldnames=columns);w.writeheader();w.writerows(rows)
def val(v): return '' if v is None else round(v,9)
def analyze(raw,cond,seed):
    people={}; events=[]; errors=[]; counts=collections.Counter()
    for row in csv.DictReader(io.StringIO(raw.decode('utf-8-sig'))):
        try:
            event=row['event']; ident=row['id'];tick=int(row['tick']);counts[event]+=1
            if event in ('initial','birth'):
                if ident in people: errors.append('duplicate_id');continue
                people[ident]={'tick':tick,'res':float(row['resistance']),'parent':row['parent_id'],'event':event,'end':None,'end_type':None}
                if event=='birth':events.append(ident)
            elif event in ('disappeared','censored'):
                if ident not in people:errors.append('end_without_origin')
                else:people[ident]['end']=tick;people[ident]['end_type']=event
        except (KeyError,ValueError):errors.append('bad_row')
    children=collections.defaultdict(list)
    for cid in events:
        p=people[cid]['parent']
        if p in people:children[p].append(cid)
        else:errors.append('missing_parent')
    # All parent events precede their children. Compute depths in birth-time order.
    depths={}; transmission=[]
    for ident,p in sorted(people.items(),key=lambda kv:kv[1]['tick']):
        if p['event']=='initial':depths[ident]=0;continue
        parent=p['parent'];pp=people.get(parent)
        if pp is None or parent not in depths:errors.append('invalid_pedigree');continue
        depths[ident]=depths[parent]+1
        transmission.append((pp['res'],p['res'],p['res']-pp['res']))
    gen=collections.defaultdict(list)
    for ident,depth in depths.items():gen[depth].append(people[ident]['res'])
    # Birth cohort mean is the mean inherited trait among new individuals, not living population mean.
    early=[people[i]['res'] for i in events if 1<=people[i]['tick']<=10000]
    late=[people[i]['res'] for i in events if 30001<=people[i]['tick']<=45000]
    # Parents with fully observable 5000-tick reproductive windows. Exclude initial population.
    eligible=[]
    for ident,p in people.items():
        if p['event']!='birth' or p['tick']>35000:continue
        if p['end_type']=='censored' and p['end'] is not None and p['end']<p['tick']+5000:continue
        if p['end'] is None:continue
        n=sum(people[c]['tick']<=p['tick']+5000 for c in children.get(ident,()))
        eligible.append((p['res'],n))
    weighted=mean([r*n for r,n in eligible]);meanres=mean([r for r,n in eligible]);totalkids=sum(n for _,n in eligible)
    selected=(weighted*len(eligible)/totalkids-meanres) if totalkids and meanres is not None else None
    high=[n for r,n in eligible if r>=.55];low=[n for r,n in eligible if r<=.45]
    rr={'condition':cond,'seed':seed,'individuals':len(people),'births':len(events),'pedigree_links':len(transmission),'max_generation':max(gen,default=0),'mean_parent_child_delta':val(mean([d for _,_,d in transmission])),'mean_abs_mutation':val(mean([abs(d) for _,_,d in transmission])),'initial_resistance':val(mean(gen.get(0,[]))),'early_newborn_resistance':val(mean(early)),'late_newborn_resistance':val(mean(late)),'late_minus_early':val((mean(late)-mean(early)) if early and late else None),'fixed_window_selection_differential':val(selected),'high_minus_low_births':val(mean(high)-mean(low) if high and low else None),'eligible_parents':len(eligible),'quality_issues':len(errors)}
    gr=[]
    for d,rs in sorted(gen.items()):
        gr.append({'condition':cond,'seed':seed,'generation':d,'n':len(rs),'mean_resistance':val(mean(rs)),'median_resistance':val(med(rs))})
    return rr,gr,errors

def main():
    a=argparse.ArgumentParser();a.add_argument('--zip',required=True,type=Path);a.add_argument('--out',required=True,type=Path);args=a.parse_args();args.out.mkdir(parents=True,exist_ok=True)
    runs=[];gens=[];issues=[]
    with zipfile.ZipFile(args.zip) as z:
        for path in z.namelist():
            if not path.endswith('.life-events.csv'):continue
            m=re.search(r'-seed(\d+)-(control|x\d+)\.life-events\.csv$',path)
            if not m:issues.append('unrecognized '+path);continue
            run,gr,errs=analyze(z.read(path),m.group(2),int(m.group(1)))
            runs.append(run);gens.extend(gr)
            issues.extend(f'{path}: {e}' for e in errs[:20])
    if not runs:raise RuntimeError('No life-events files')
    out(args.out/'generation_runs.csv',runs,list(runs[0]))
    out(args.out/'generation_depths.csv',gens,list(gens[0]))
    summary=[]
    for cond in ['control','x1','x2','x4','x10','x20','x50']:
        rr=[r for r in runs if r['condition']==cond]
        def vals(key):return [r[key] for r in rr if isinstance(r[key],(float,int)) and r[key]!='']
        summary.append({'condition':cond,'runs':len(rr),'median_late_minus_early':val(med(vals('late_minus_early'))),'positive_late_minus_early':sum(v>0 for v in vals('late_minus_early')),'median_selection_differential':val(med(vals('fixed_window_selection_differential'))),'positive_selection_differential':sum(v>0 for v in vals('fixed_window_selection_differential')),'median_max_generation':val(med(vals('max_generation'))),'median_parent_child_delta':val(med(vals('mean_parent_child_delta'))),'total_quality_issues':sum(r['quality_issues'] for r in rr)})
    out(args.out/'summary.csv',summary,list(summary[0]))
    by={(r['condition'],r['seed']):r for r in runs};pairs=[]
    for (cond,seed),r in by.items():
        if cond!='x10' or ('control',seed) not in by:continue
        b=by['control',seed]
        pairs.append({'seed':seed,'control_late_minus_early':b['late_minus_early'],'x10_late_minus_early':r['late_minus_early'],'x10_minus_control':val(r['late_minus_early']-b['late_minus_early']) if isinstance(r['late_minus_early'],float) and isinstance(b['late_minus_early'],float) else ''})
    out(args.out/'paired_x10_vs_control.csv',pairs,list(pairs[0]))
    x=next(s for s in summary if s['condition']=='x10')
    paired=[p['x10_minus_control'] for p in pairs if isinstance(p['x10_minus_control'],float)]
    report=f'''# Phase 07.2.28 世代別毒耐性と遺伝的選択の検証\n\n- 解析ラン: {len(runs)}\n- 個体履歴の親子リンク総数: {sum(r['pedigree_links'] for r in runs):,}\n- 毒10倍: 出生時期の後期−前期の耐性差中央値 {x['median_late_minus_early']}、正のラン {x['positive_late_minus_early']}/{x['runs']}\n- 毒10倍: 5,000 Tickの繁殖選択差中央値 {x['median_selection_differential']}、正のラン {x['positive_selection_differential']}/{x['runs']}\n- 毒10倍−対照: 同seedの出生時期差の差中央値 {val(med(paired))}\n- 品質警告数: {len(issues)}\n\n## 指標の解釈\n- generation_depths.csv は初期個体を世代0とした系譜深度であり、同一Tickの世代や生存個体の平均ではない。\n- late_minus_early は1〜10,000 Tick出生と30,001〜45,000 Tick出生の個体の平均耐性差。個体数による重みの影響がある。\n- fixed_window_selection_differential は、5,000 Tickで子を多く残した親の耐性平均と対象親の耐性平均との差。観察的関連であり、突然変異・出生時期・環境差を分離した因果推定ではない。\n- 親子の耐性差は遺伝・突然変異の観測であり、世代別耐性上昇が自然選択のみで生じた証明にはならない。\n- 既存CSVには個体の死因や全時点の生存個体の耐性分布がない。生存個体群の耐性変化や選択の因果効果は未検証。\n- 07.2.25の自然発芽引数不整合が指摘されているため、設定の同等性を別途確認する必要がある。\n'''
    (args.out/'REPORT.md').write_text(report,encoding='utf-8')
    (args.out/'quality.txt').write_text('OK\n' if not issues else '\n'.join(issues[:200]),encoding='utf-8')
    print(report)
if __name__=='__main__':main()
