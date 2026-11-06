#!/usr/bin/env python3
"""Phase 07.2.11: descriptive diagnostics of between-seed heterogeneity.
No causal attribution; original five seeds lack cohort-level diagnostics in this input.
"""
import argparse,csv,json,math,statistics,hashlib
from pathlib import Path

def read(p,fields):
    with p.open(encoding='utf-8-sig',newline='') as f:
        r=csv.DictReader(f)
        missing=set(fields)-set(r.fieldnames or [])
        if missing: raise ValueError(f'{p}: missing {sorted(missing)}')
        return list(r)

def write(p,rows,cols):
    with p.open('w',encoding='utf-8',newline='') as f:
        w=csv.DictWriter(f,fieldnames=cols);w.writeheader();w.writerows(rows)

def pearson(x,y):
    if len(x)<4:return None
    mx=statistics.mean(x);my=statistics.mean(y)
    den=math.sqrt(sum((v-mx)**2 for v in x)*sum((v-my)**2 for v in y))
    return sum((a-mx)*(b-my) for a,b in zip(x,y))/den if den>0 else None

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--paired',default='results/phase07210/analysis/design/paired_seed_summary.csv')
    ap.add_argument('--cohorts',default='results/phase07210/analysis/cohort_comparisons.csv')
    ap.add_argument('--out',default='results/phase07211/analysis')
    a=ap.parse_args(); pp=Path(a.paired);cp=Path(a.cohorts);out=Path(a.out);out.mkdir(parents=True,exist_ok=True)
    paired=read(pp,['seed','source','B_survival_diff','F_survival_diff','F_minus_B_survival','F_minus_B_reproduction'])
    cohort=read(cp,['run','cohort','n_low','n_high','survival_diff','reproduction_diff','toxin_dose_diff','toxin_damage_diff','prevented_damage_diff','resistance_cost_diff'])
    seen=set(); vals=[]; issues=[]
    for r in paired:
        seed=int(r['seed'])
        if seed in seen:raise ValueError(f'duplicate seed {seed}')
        seen.add(seed)
        for k in ['B_survival_diff','F_survival_diff','F_minus_B_survival','F_minus_B_reproduction']:
            if not math.isfinite(float(r[k])):raise ValueError(f'nonfinite {seed} {k}')
        if abs(float(r['F_survival_diff'])-float(r['B_survival_diff'])-float(r['F_minus_B_survival']))>1e-8:
            raise ValueError(f'inconsistent contrast {seed}')
        vals.append((seed,float(r['F_minus_B_survival'])))
    if len(vals)<3:raise ValueError('at least 3 paired seeds required')
    mu=statistics.mean(v for _,v in vals)
    influence=[]
    for seed,v in vals:
        loo=statistics.mean(z for s,z in vals if s!=seed)
        influence.append(dict(seed=seed,contrast=v,leave_one_out_mean=loo,change_vs_full=loo-mu))
    write(out/'leave_one_seed_out.csv',influence,['seed','contrast','leave_one_out_mean','change_vs_full'])
    grouped={}
    for r in cohort:
        run=r['run']
        if '-seed' not in run:raise ValueError(f'bad run {run}')
        cond,seedstr=run.rsplit('-seed',1);seed=int(seedstr)
        if cond not in ('B_no_toxin_cost','F_strong_cost'):raise ValueError(f'unexpected condition {cond}')
        nl=int(r['n_low']);nh=int(r['n_high'])
        if nl<=0 or nh<=0:raise ValueError(f'nonpositive cohort size {run}')
        for k in ['survival_diff','reproduction_diff','toxin_dose_diff','toxin_damage_diff','prevented_damage_diff','resistance_cost_diff']:
            if not math.isfinite(float(r[k])):raise ValueError(f'nonfinite {run} {k}')
        grouped.setdefault((seed,cond),[]).append((min(nl,nh),r))
    rows=[]
    for (seed,cond),rr in sorted(grouped.items()):
        total=sum(w for w,_ in rr)
        result={'seed':seed,'condition':cond,'cohorts':len(rr),'effective_weight':total,
                'min_cohort_weight':min(w for w,_ in rr),'max_cohort_weight':max(w for w,_ in rr)}
        result['max_cohort_weight_share']=result['max_cohort_weight']/total
        for col in ['survival_diff','reproduction_diff','toxin_dose_diff','toxin_damage_diff','prevented_damage_diff','resistance_cost_diff']:
            result[col]=sum(w*float(r[col]) for w,r in rr)/total
        rows.append(result)
    cols=list(rows[0]) if rows else []
    if not rows:raise ValueError('no cohort rows')
    write(out/'seed_cohort_diagnostics.csv',rows,cols)
    pairs={}
    for r in rows:pairs.setdefault(r['seed'],{})[r['condition']]=r
    joined=[]
    for seed,conditions in sorted(pairs.items()):
        if len(conditions)!=2:
            issues.append(f'new seed {seed} missing B or F cohort diagnostics');continue
        b=conditions['B_no_toxin_cost'];f=conditions['F_strong_cost']
        joined.append({'seed':seed,'F_minus_B_survival':f['survival_diff']-b['survival_diff'],
          'F_minus_B_reproduction':f['reproduction_diff']-b['reproduction_diff'],
          'B_cohorts':b['cohorts'],'F_cohorts':f['cohorts'],
          'B_weight':b['effective_weight'],'F_weight':f['effective_weight'],
          'B_largest_cohort_share':b['max_cohort_weight_share'],
          'F_largest_cohort_share':f['max_cohort_weight_share'],
          'F_toxin_dose_diff':f['toxin_dose_diff'],
          'F_toxin_damage_diff':f['toxin_damage_diff'],
          'F_prevented_damage_diff':f['prevented_damage_diff'],
          'B_resistance_cost_diff':b['resistance_cost_diff'],
          'F_resistance_cost_diff':f['resistance_cost_diff']})
    write(out/'new_seed_covariates.csv',joined,list(joined[0]) if joined else ['seed'])
    metrics=[k for k in joined[0] if k not in ('seed','F_minus_B_survival','F_minus_B_reproduction')] if joined else []
    corrs=[]
    for k in metrics:
        rho=pearson([r[k] for r in joined],[r['F_minus_B_survival'] for r in joined])
        corrs.append({'covariate':k,'n_seeds':len(joined),'pearson_r':rho if rho is not None else ''})
    write(out/'exploratory_correlations.csv',corrs,['covariate','n_seeds','pearson_r'])
    sources={r['source'] for r in paired}
    if 'original' in sources:issues.append('Original 5 seeds have only paired summaries; no original cohort-level covariates in this input')
    issues.append('Cumulative dose/cost and cohort sizes may be consequences of survival; correlations are non-causal')
    issues.append('No individual population size, predation pressure, or initial allele distribution in supplied aggregate tables')
    issues.append('Exploratory multiple correlations; no significance testing or causal interpretation')
    quality={'paired_seeds':len(paired),'new_cohort_seed_pairs':len(joined),'new_cohort_rows':len(cohort),
             'issues':issues,'paired_sha256':hashlib.sha256(pp.read_bytes()).hexdigest(),
             'cohorts_sha256':hashlib.sha256(cp.read_bytes()).hexdigest()}
    (out/'quality.json').write_text(json.dumps(quality,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
    top=sorted(influence,key=lambda r:abs(r['change_vs_full']),reverse=True)[:5]
    lines=['# Phase 07.2.11 — 選択圧のばらつきの原因分析（探索的）','',
       f'- 既存＋追加ペアシード: {len(paired)}。F−B平均生存率差: {mu*100:+.3f} pt。',
       f'- 新規コホート詳細があるシードペア: {len(joined)}。','',
       '## 1シード除外感度（平均差の変化が大きい上位）','',
       '| Seed | F−B (pt) | 除外後平均 (pt) | 平均変化 (pt) |','|---|---:|---:|---:|']
    for r in top:lines.append(f"| {r['seed']} | {r['contrast']*100:+.3f} | {r['leave_one_out_mean']*100:+.3f} | {r['change_vs_full']*100:+.3f} |")
    lines+=['','## 新規15シードの探索的相関','',
      '相関はシード単位の記述統計であり、原因を確定しません。特に曝露量と維持コストは生存期間に依存します。','',
      '| 指標 | Pearson r |','|---|---:|']
    for r in sorted(corrs,key=lambda z:abs(z['pearson_r']) if isinstance(z['pearson_r'],float) else -1,reverse=True):
        v=r['pearson_r'];lines.append(f"| {r['covariate']} | {v:+.3f} |" if isinstance(v,float) else f"| {r['covariate']} | 算出不可 |")
    lines+=['','## 判定','',
      '- Phase 07.2.11の記述的診断は完了。原因の因果的同定は未達。',
      '- 元CSVの再送・Rust再実行は不要。未観測の捕食圧・初期耐性分布は今回の表から推測しない。',
      '- 次段階で個体群規模・初期遺伝子分布・捕食圧のシード別要約をローカル抽出し、共有するのが望ましい。','',
      '## データ制約','']+['- '+x for x in issues]
    (out/'REPORT.md').write_text('\n'.join(lines)+'\n',encoding='utf-8')
    print(f'Completed: {len(paired)} paired seeds; {len(joined)} new detailed pairs; output={out}')
if __name__=='__main__':main()
