#!/usr/bin/env python3
"""Phase 07.2.14: descriptive birth pause and recovery diagnostics.
Only interval aggregates; cannot infer fertility constraints or individual causes.
"""
import argparse,csv,json,math,statistics,collections
from pathlib import Path
CONDS=('B_no_toxin_cost','F_strong_cost')
FIELDS=('births','deaths','toxin_events','hunt_attempts')

def read(p):
 with p.open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def write(p,rows,fields):
 with p.open('w',encoding='utf-8',newline='') as f:
  w=csv.DictWriter(f,fieldnames=fields,extrasaction='ignore');w.writeheader();w.writerows(rows)
def avg(v):return round(statistics.mean(v),6) if v else None
def rate(num,exposure):return round(1000*num/exposure,6) if exposure>0 else None

def analyze(input_dir,out,expected=50000,step=100,early=(3000,6000),recovery=(6000,10000)):
 out.mkdir(parents=True,exist_ok=True)
 issues=[];data={};summaries=[];intervals=[];paired=[];groups=[]
 path=input_dir/'interval_rates.csv'
 if not path.exists():raise FileNotFoundError(path)
 raw=read(path)
 for line,r in enumerate(raw,2):
  try:
   s=int(r['seed']);c=r['condition'];a=int(r['start_tick']);b=int(r['end_tick'])
   if c not in CONDS:raise ValueError('unexpected condition')
   if b-a!=step or a<0 or b>expected:raise ValueError('tick interval mismatch')
   d={'seed':s,'condition':c,'start_tick':a,'end_tick':b,'pop_start':int(r['herbivores_start']),'pop_end':int(r['herbivores_end'])}
   for f in FIELDS:d[f]=float(r[f])
   for f in ('pop_start','pop_end',*FIELDS):
    if not math.isfinite(d[f]) or d[f]<0:raise ValueError('invalid '+f)
   if any(abs(d[f]-round(d[f]))>1e-5 for f in ('births','deaths','toxin_events','hunt_attempts')):raise ValueError('noninteger counter')
   if d['pop_end']-d['pop_start']!=round(d['births']-d['deaths']):raise ValueError('population balance mismatch')
   data.setdefault((s,c),[]).append(d)
  except (ValueError,KeyError) as e:issues.append(f'line {line}: {e}')
 for key,rs in sorted(data.items()):
  s,c=key;rs.sort(key=lambda r:r['start_tick'])
  if len(rs)!=expected//step or rs[0]['start_tick']!=0 or rs[-1]['end_tick']!=expected:issues.append(f'{key}: incomplete timeline')
  for a,b in zip(rs,rs[1:]):
   if a['end_tick']!=b['start_tick'] or a['pop_end']!=b['pop_start']:issues.append(f'{key}: timeline gap/discontinuity at {b["start_tick"]}')
  # First *consecutive* zero-birth run after tick 3000, while a population remains.
  zero_streaks=[];start=None;end=None
  for r in rs:
   eligible=r['start_tick']>=early[0] and r['end_tick']<=early[1] and r['pop_start']>0 and r['births']==0
   if eligible:
    if start is None:start=r['start_tick']
    end=r['end_tick']
   elif start is not None:
    zero_streaks.append((start,end));start=end=None
  if start is not None:zero_streaks.append((start,end))
  longest=max(zero_streaks,key=lambda x:x[1]-x[0]) if zero_streaks else (None,None)
  def aggregate(start,end):
   selected=[r for r in rs if start<=r['start_tick'] and r['end_tick']<=end]
   if len(selected)!=(end-start)//step:return None
   exp=sum((r['pop_start']+r['pop_end'])/2*step for r in selected)
   return {'births':sum(r['births'] for r in selected),'deaths':sum(r['deaths'] for r in selected),'toxin_events':sum(r['toxin_events'] for r in selected),'hunt_attempts':sum(r['hunt_attempts'] for r in selected),'pop_start':selected[0]['pop_start'],'pop_end':selected[-1]['pop_end'],'exposure':exp,'birth_rate':rate(sum(r['births'] for r in selected),exp),'death_rate':rate(sum(r['deaths'] for r in selected),exp),'toxin_rate':rate(sum(r['toxin_events'] for r in selected),exp),'hunt_rate':rate(sum(r['hunt_attempts'] for r in selected),exp)}
  early_a=aggregate(*early);recovery_a=aggregate(*recovery)
  if early_a is None or recovery_a is None:issues.append(f'{key}: missing early/recovery window');continue
  firstzero=next((r['end_tick'] for r in rs if r['pop_end']==0),None)
  # Observed recovery, operationally: population at 10k > at 6k, with nonzero at 6k.
  # If zero at 6k, separately track revival; do not call irreversible extinction.
  recovery_status=('revival_from_zero' if recovery_a['pop_start']==0 and recovery_a['pop_end']>0 else
                   'zero_at_both' if recovery_a['pop_start']==0 else
                   'increase' if recovery_a['pop_end']>recovery_a['pop_start'] else
                   'flat' if recovery_a['pop_end']==recovery_a['pop_start'] else 'decrease')
  d={'seed':s,'condition':c,'first_zero_tick':firstzero,'final_zero':int(rs[-1]['pop_end']==0),'zero_birth_longest_start':longest[0],'zero_birth_longest_end':longest[1],'zero_birth_longest_ticks':longest[1]-longest[0] if longest[0] is not None else 0,'early_births':early_a['births'],'early_deaths':early_a['deaths'],'early_birth_rate':early_a['birth_rate'],'early_death_rate':early_a['death_rate'],'recovery_start_population':recovery_a['pop_start'],'recovery_end_population':recovery_a['pop_end'],'recovery_status':recovery_status,'recovery_births':recovery_a['births'],'recovery_deaths':recovery_a['deaths'],'recovery_birth_rate':recovery_a['birth_rate'],'recovery_death_rate':recovery_a['death_rate'],'recovery_toxin_rate':recovery_a['toxin_rate'],'recovery_hunt_rate':recovery_a['hunt_rate']}
  summaries.append(d)
  # Fixed 1000-tick windows: event timing, exposure and demography; avoids overlapping windows.
  for st in range(0,expected,1000):
   a=aggregate(st,st+1000)
   if a is None:issues.append(f'{key}: missing 1000-tick window {st}');continue
   intervals.append({'seed':s,'condition':c,'start_tick':st,'end_tick':st+1000,'herbivores_start':a['pop_start'],'herbivores_end':a['pop_end'],'births':a['births'],'deaths':a['deaths'],'births_per_1000_herbivore_ticks':a['birth_rate'],'deaths_per_1000_herbivore_ticks':a['death_rate'],'toxin_events_per_1000_herbivore_ticks':a['toxin_rate'],'hunt_attempts_per_1000_herbivore_ticks':a['hunt_rate']})
 by={(r['seed'],r['condition']):r for r in summaries}
 seeds=sorted(set(s for s,c in data))
 for s in seeds:
  b=by.get((s,CONDS[0]));f=by.get((s,CONDS[1]))
  if not b or not f:issues.append(f'seed {s}: missing pair');continue
  paired.append({'seed':s,'B_recovery_status':b['recovery_status'],'F_recovery_status':f['recovery_status'],'B_population_10000':b['recovery_end_population'],'F_population_10000':f['recovery_end_population'],'F_minus_B_population_10000':f['recovery_end_population']-b['recovery_end_population'],'B_recovery_birth_rate':b['recovery_birth_rate'],'F_recovery_birth_rate':f['recovery_birth_rate'],'B_recovery_death_rate':b['recovery_death_rate'],'F_recovery_death_rate':f['recovery_death_rate'],'B_final_zero':b['final_zero'],'F_final_zero':f['final_zero']})
 for c in CONDS:
  rr=[r for r in summaries if r['condition']==c]
  groups.append({'condition':c,'runs':len(rr),'final_zero_runs':sum(r['final_zero'] for r in rr),'recovery_increase_runs':sum(r['recovery_status']=='increase' for r in rr),'recovery_decrease_runs':sum(r['recovery_status']=='decrease' for r in rr),'recovery_zero_at_both_runs':sum(r['recovery_status']=='zero_at_both' for r in rr),'mean_longest_zero_birth_ticks':avg([r['zero_birth_longest_ticks'] for r in rr]),'mean_population_10000':avg([r['recovery_end_population'] for r in rr]),'mean_recovery_birth_rate':avg([r['recovery_birth_rate'] for r in rr if r['recovery_birth_rate'] is not None]),'mean_recovery_death_rate':avg([r['recovery_death_rate'] for r in rr if r['recovery_death_rate'] is not None])})
 if len(summaries)!=40:issues.append(f'{len(summaries)} runs, expected 40')
 if len(paired)!=20:issues.append(f'{len(paired)} pairs, expected 20')
 for name,rows,cols in [('run_recovery.csv',summaries,list(summaries[0]) if summaries else ['seed']),('paired_recovery.csv',paired,list(paired[0]) if paired else ['seed']),('condition_summary.csv',groups,list(groups[0]) if groups else ['condition']),('fixed_window_rates.csv',intervals,list(intervals[0]) if intervals else ['seed'])]:write(out/name,rows,cols)
 q={'condition_runs':len(summaries),'paired_seeds':len(paired),'expected_ticks':expected,'sampling_interval':step,'birth_pause_window':list(early),'recovery_window':list(recovery),'issues':issues,'limitations':['Zero births in a 100-tick interval does not prove reproduction is disabled; small population and stochastic variation can produce zero births.','A zero-birth streak is only sought during the fixed 3000-6000 tick window, and intervals starting with zero population are excluded.','Recovery means population increase from tick 6000 to 10000, not return to a pre-decline baseline.','Rates are based on trapezoidal herbivore-time exposure, not individual-level risk or cause-specific mortality.','No direct observation of fertility eligibility, age, satiety, resource limitations, or death causes; mechanism cannot be identified.','Predator hunt attempts are not predator-caused deaths; toxin exposure is not toxin-caused death.','Comparisons are descriptive/exploratory and do not establish causality or natural selection.']}
 (out/'quality.json').write_text(json.dumps(q,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
 report=['# Phase 07.2.14 — 繁殖停止と急減後の回復診断','',f'- 対象: {len(summaries)}ラン / 40、{len(paired)}ペア / 20',f'- 品質問題: {len(issues)}件','- 既存Phase 07.2.13の100 Tick区間集計から再解析。シミュレーション再実行なし。','','## 条件別集計']
 for g in groups:report.append(f'- {g["condition"]}: 最終個体数ゼロ {g["final_zero_runs"]}/{g["runs"]}、6,000→10,000 Tickで増加 {g["recovery_increase_runs"]}、減少 {g["recovery_decrease_runs"]}、両時点ゼロ {g["recovery_zero_at_both_runs"]}、3,000〜6,000 Tick内の最長無出生区間の平均 {g["mean_longest_zero_birth_ticks"]} Tick')
 report+=['','## 解釈','- 出生ゼロが共通して観測されても、繁殖機構の停止・原因は判定できない。','- 6,000→10,000 Tickの個体数変化は回復の操作的指標であり、長期存続とは別。','- 毒・捕食の累積イベントと死亡原因は区別する。','- 追加の原因特定には繁殖可能個体数、年齢構成、繁殖条件不成立理由、死因別カウンタが必要。','', '## 品質問題']
 report.extend(['- '+x for x in issues] if issues else ['- なし'])
 (out/'REPORT.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
 return q

def main():
 p=argparse.ArgumentParser();p.add_argument('--input',default='results/phase07213/analysis');p.add_argument('--out',default='results/phase07214/analysis');p.add_argument('--expected-ticks',type=int,default=50000);p.add_argument('--interval',type=int,default=100)
 a=p.parse_args()
 if a.expected_ticks<10000 or a.interval<1 or a.expected_ticks%a.interval:p.error('invalid ticks/interval')
 q=analyze(Path(a.input),Path(a.out),a.expected_ticks,a.interval)
 print(f'Runs={q["condition_runs"]} pairs={q["paired_seeds"]} issues={len(q["issues"])}')
 if q['issues']:raise SystemExit(2)
if __name__=='__main__':main()
