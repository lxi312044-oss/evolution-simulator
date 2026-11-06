#!/usr/bin/env python3
"""Phase 07.2.9: prospective planning only, not a confirmatory power guarantee."""
import argparse,csv,math,statistics,json,hashlib
from pathlib import Path
from statistics import NormalDist
CONDITIONS=['A_no_toxin_no_cost','B_no_toxin_cost','C_toxin_no_cost','D_toxin_cost','E_strong_no_cost','F_strong_cost']
PAIRS=[('B-A','B_no_toxin_cost','A_no_toxin_no_cost'),('C-A','C_toxin_no_cost','A_no_toxin_no_cost'),('D-B','D_toxin_cost','B_no_toxin_cost'),('E-A','E_strong_no_cost','A_no_toxin_no_cost'),('F-B','F_strong_cost','B_no_toxin_cost'),('D-C','D_toxin_cost','C_toxin_no_cost'),('F-E','F_strong_cost','E_strong_no_cost')]
def read(path,fields):
 with path.open(encoding='utf-8-sig',newline='') as f:
  r=csv.DictReader(f)
  if not set(fields).issubset(r.fieldnames or []):raise ValueError(f'{path}: missing {sorted(set(fields)-set(r.fieldnames or []))}')
  return list(r)
def write(path,rows,fields):
 with path.open('w',encoding='utf-8',newline='') as f:
  w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(rows)
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--input',default='results/phase0728/analysis/seed_summary.csv')
 p.add_argument('--out',default='results/phase0729/analysis')
 p.add_argument('--alpha',type=float,default=.05)
 p.add_argument('--power',type=float,default=.8)
 p.add_argument('--effects',default='0.01,0.02,0.05',help='absolute survival probability differences; planning scenarios only')
 a=p.parse_args()
 if not 0<a.alpha<1 or not 0<a.power<1: p.error('alpha and power must be in (0,1)')
 try: effects=[float(x) for x in a.effects.split(',')]
 except ValueError:p.error('invalid effects')
 if not effects or any(not math.isfinite(x) or x<=0 for x in effects):p.error('effects must be positive finite numbers')
 path=Path(a.input);out=Path(a.out);out.mkdir(parents=True,exist_ok=True)
 rows=read(path,['condition','seed','cohorts','effective_matched_weight','survival_diff','reproduction_diff'])
 data={}; warnings=[]
 for i,r in enumerate(rows,2):
  try:
   key=(r['condition'],r['seed']);val=float(r['survival_diff']);birth=float(r['reproduction_diff']);coh=int(r['cohorts']);weight=float(r['effective_matched_weight'])
   if key in data or not math.isfinite(val) or not math.isfinite(birth) or coh<1 or weight<=0:raise ValueError('duplicate/nonfinite/nonpositive')
   data[key]={'survival':val,'reproduction':birth,'cohorts':coh,'weight':weight}
  except ValueError as e:raise SystemExit(f'Invalid input row {i}: {e}')
 if not data:raise SystemExit('No seed summaries')
 z=NormalDist().inv_cdf(1-a.alpha/2)+NormalDist().inv_cdf(a.power)
 fields=['comparison','metric','n_pilot_seeds','pilot_mean','pilot_sd','target_effect','approx_required_seeds','method','limitations']
 result=[];summary=[]
 for metric in ['survival','reproduction']:
  comparisons=[(c,[v[metric] for (cc,s),v in data.items() if cc==c]) for c in CONDITIONS]
  for name,c1,c0 in PAIRS:
   seeds=sorted(set(s for c,s in data if c==c1)&set(s for c,s in data if c==c0))
   comparisons.append((name,[data[(c1,s)][metric]-data[(c0,s)][metric] for s in seeds]))
  for name,vals in comparisons:
   n=len(vals);sd=statistics.stdev(vals) if n>=2 else None;mean=statistics.mean(vals) if vals else None
   if n<5:warnings.append(f'{name}/{metric}: fewer than five pilot seeds')
   if sd is not None and sd==0:warnings.append(f'{name}/{metric}: zero pilot variance; no reliable estimate')
   for effect in effects:
    estimate=math.ceil((z*sd/effect)**2) if sd is not None and sd>0 else None
    # At least 2 independent seeds needed for variance estimation; never promise sufficiency.
    estimate=max(2,estimate) if estimate is not None else None
    result.append(dict(comparison=name,metric=metric,n_pilot_seeds=n,pilot_mean=mean,pilot_sd=sd,target_effect=effect,approx_required_seeds=estimate,method='two-sided normal approximation; independent seed means',limitations='5-seed SD unstable; ignores selection bias, multiple testing, power loss from unmatched cohorts'))
   summary.append((name,metric,n,mean,sd))
 write(out/'planning_scenarios.csv',result,fields)
 quality={'pilot_runs':len(data),'unique_seeds':len(set(s for c,s in data)),'conditions':{c:sum(cc==c for cc,s in data) for c in CONDITIONS},'warnings':warnings,'input_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'alpha':a.alpha,'target_power':a.power,'effect_scenarios':effects}
 (out/'quality.json').write_text(json.dumps(quality,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
 report=['# Phase 07.2.9 — 自然選択の検出力改善に向けた実験設計','','## 設計の位置付け','- **今回の数値は正式な検出力保証ではなく、追加実験を計画するための概算です。**','- 生態・遺伝・毒のパラメータを変更しない。','- 独立反復単位はシード。個体数・コホート数を独立反復として水増ししない。','- 主要候補指標: 出生後1000 Tickの高耐性−低耐性生存率差（条件間の差の差も探索的に扱う）。','- 繁殖数差は副次指標。複数条件・複数指標の確証的検定をする場合は事前に多重性を定義する。','- 目標効果量 1/2/5パーセントポイントは**設計シナリオ**であり、生物学的に意味のある閾値として未承認。','- α=%.3f、目標検出力=%.2f、両側正規近似。'%(a.alpha,a.power),'','## 予備データ（シード間標準偏差）','','| 比較 | 指標 | n | 平均差 | シードSD |','|---|---|---:|---:|---:|']
 for name,metric,n,mean,sd in summary:report.append(f'| {name} | {metric} | {n} | {mean:+.5f} | {sd:.5f} |' if sd is not None else f'| {name} | {metric} | {n} | NA | NA |')
 report+=['','## 概算シード数','- `planning_scenarios.csv` に全条件・全ペアの必要数の概算を出力。','- 算式は ceil(((z(1−α/2)+z(power)) × pilot_SD / target_effect)^2)。最低2シードと表示するが、2シードで推定が安定する意味ではない。','- 5シードのSD推定は極めて不安定。観測群の選択バイアスや条件間乱数差は解消されない。','- 予備実験の追加（例: まず独立シードを合計20程度へ）と分散再推定を推奨するが、**正式なシード数は効果量・計算予算の承認後に確定**。','','## 事前登録する判定規則（提案・未確定）','1. 主要比較・主要指標・効果量閾値を事前に一つ定める。','2. 追加シードの範囲とシード採番規則を実行前に固定し、途中の結果を見て有意になるまで追加しない。','3. コホート比較不能のシード・欠損・絶滅時の扱いを事前に固定する。','4. 効果の方向・区間推定・独立シード間の再現性を併記し、非有意を「自然選択なし」と解釈しない。','5. 主要比較が確定するまでは、A〜Fの全比較を探索的に扱う。','','## 継続課題','- 観測上の高/低耐性群は無作為割付ではないため、因果効果の直接推定はできない。','- 追加シードの独立性、実行コスト、コホート数の不足を検証する。','- 固定期間の繁殖・毒曝露の測定仕様と、世代間遺伝子頻度の評価を別途事前定義する。','- 生物学的に意味のある最小効果量の決定はユーザー確認待ち。','','## 品質','- 入力SHA256: `'+quality['input_sha256']+'`','- 入力シード集計行数: '+str(len(data)),'- 警告: '+str(len(warnings))]
 (out/'REPORT.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
 print(f'Wrote {len(result)} scenarios to {out}; warnings={len(warnings)}')
if __name__=='__main__':main()
