#!/usr/bin/env python3
"""Phase 07.2.22.1: recover malformed 07.2.22 IDs; conservative life-history diagnostics."""
import argparse,csv,io,json,re,statistics,zipfile,math
from pathlib import Path
from collections import Counter,defaultdict
ID=re.compile(r'^EntityId \{ index: (\d+), generation: (\d+) \}$')
COLUMNS='seed,tick,event,id,parent_id,resistance,age,hp,lifetime_toxin_dose,lifetime_toxin_damage,lifetime_prevented_damage,lifetime_resistance_cost,successful_births,mutation_attempted'.split(',')
VALID={'initial','birth','disappeared','censored'}
def normalize_id(s):
 s=s.strip()
 if not s:return ''
 m=ID.fullmatch(s)
 if m:return ':'.join(m.groups())
 if re.fullmatch(r'\d+:\d+',s):return s
 raise ValueError('invalid ID '+repr(s))
def parse_row(line):
 # Only repair the exact legacy unquoted EntityId form, never guess arbitrary CSV.
 line=re.sub(r'EntityId \{ index: (\d+), generation: (\d+) \}',lambda m:m.group(1)+':'+m.group(2),line)
 row=next(csv.reader([line]))
 if len(row)!=len(COLUMNS):raise ValueError(f'column count {len(row)} != {len(COLUMNS)}')
 d=dict(zip(COLUMNS,row));d['id']=normalize_id(d['id']);d['parent_id']=normalize_id(d['parent_id'])
 if d['event'] not in VALID:raise ValueError('invalid event')
 for field in ('seed','tick','age','successful_births'):
  if d[field] and not d[field].isdigit():raise ValueError('invalid integer '+field)
 for field in ('resistance','hp','lifetime_toxin_dose','lifetime_toxin_damage','lifetime_prevented_damage','lifetime_resistance_cost'):
  if d[field] and not (math.isfinite(float(d[field])) or d[field].lower()=='nan'):raise ValueError('invalid float '+field)
 if not d['id']:raise ValueError('missing id')
 return d
def write_csv(path,rows,columns):
 with path.open('w',encoding='utf-8',newline='') as f:
  w=csv.DictWriter(f,fieldnames=columns,extrasaction='ignore');w.writeheader();w.writerows(rows)
def median(values):return statistics.median(values) if values else None
def main():
 p=argparse.ArgumentParser()
 p.add_argument('--zip',help='Original phase07222.zip; no re-simulation required')
 p.add_argument('--root',default='results/phase07222/runs')
 p.add_argument('--out',default='results/phase072221/analysis')
 p.add_argument('--expected-seeds',type=int,default=20)
 p.add_argument('--ticks',type=int,default=50000)
 a=p.parse_args();out=Path(a.out);out.mkdir(parents=True,exist_ok=True)
 issues=[];summaries=[];individuals=[];event_count=Counter();seen_runs=set();group_rows=defaultdict(list)
 if a.zip:
  z=zipfile.ZipFile(a.zip)
  sources=[(n,lambda n=n:z.open(n)) for n in z.namelist() if n.endswith('.life-events.csv')]
 else:
  sources=[(str(f),lambda f=f:f.open('rb')) for f in sorted(Path(a.root).glob('*.life-events.csv'))]
 for name,opener in sources:
  stem=Path(name).name.removesuffix('.life-events.csv')
  m=re.fullmatch(r'(B_no_toxin_cost|F_strong_cost)-seed(\d+)-(control|x1|x2|x4)',stem)
  if not m:issues.append(stem+': invalid run name');continue
  condition,seedstr,level=m.groups();seed=int(seedstr)
  if (condition,level) not in [('B_no_toxin_cost','control'),('F_strong_cost','x1'),('F_strong_cost','x2'),('F_strong_cost','x4')]:issues.append(stem+': invalid group');continue
  key=(condition,seed,level)
  if key in seen_runs:issues.append(stem+': duplicate run');continue
  seen_runs.add(key)
  events=defaultdict(dict);birth_parent=Counter();parents=defaultdict(set);rows=0;malformed=0
  with opener() as raw:
   stream=io.TextIOWrapper(raw,encoding='utf-8-sig',newline='')
   header=stream.readline().strip('\r\n').split(',')
   if header!=COLUMNS:issues.append(stem+': header mismatch');continue
   for lineno,line in enumerate(stream,2):
    if not line.strip():continue
    if 'EntityId {' in line:malformed+=1
    try:
     d=parse_row(line)
     if int(d['seed'])!=seed:raise ValueError('seed mismatch')
     if int(d['tick'])>a.ticks:raise ValueError('tick out of range')
     event=d['event'];ident=d['id'];tick=int(d['tick']);rows+=1;event_count[event]+=1
     if event in events[ident]:raise ValueError('duplicate '+event+' for '+ident)
     events[ident][event]=d
     if event=='birth':
      parent=d['parent_id']
      if not parent:raise ValueError('birth missing parent')
      birth_parent[parent]+=1;parents[parent].add(ident)
    except (ValueError,csv.Error) as e:
     issues.append(f'{stem}:{lineno}: {e}')
  censored=0;complete=0;unknown=0;offspring_disagreements=0
  for ident,ev in events.items():
   origin=ev.get('birth') or ev.get('initial')
   end=ev.get('disappeared') or ev.get('censored')
   if not origin:issues.append(stem+': terminal-only ID '+ident);continue
   if 'birth' in ev and 'initial' in ev:issues.append(stem+': conflicting origin '+ident)
   if 'disappeared' in ev and 'censored' in ev:issues.append(stem+': conflicting terminal '+ident)
   if end:
    status='observed_disappearance' if 'disappeared' in ev else 'right_censored'
    complete+=status=='observed_disappearance';censored+=status=='right_censored'
   else:status='unobserved_at_end';unknown+=1
   birth_tick=int(origin['tick']);end_tick=int(end['tick']) if end else None
   if end_tick is not None and end_tick<birth_tick:issues.append(stem+': terminal before origin '+ident)
   age_at_end=int(end['age']) if end and end['age'] else None
   age_at_start=int(origin['age']) if origin['age'] else 0
   recorded_births=int(end['successful_births']) if end and end['successful_births'] else None
   observed_births=birth_parent[ident]
   if recorded_births is not None and recorded_births!=observed_births:offspring_disagreements+=1
   resistance=float(origin['resistance'])
   group='low' if resistance<0.4 else 'mid' if resistance<0.6 else 'high'
   row=dict(condition=condition,seed=seed,level=level,id=ident,origin=origin['event'],origin_tick=birth_tick,terminal_tick=end_tick,status=status,resistance_at_origin=resistance,resistance_bin=group,age_at_origin=age_at_start,age_at_terminal=age_at_end,observed_children=observed_births,recorded_successful_births=recorded_births,toxin_dose_at_terminal=end['lifetime_toxin_dose'] if end else '',toxin_damage_at_terminal=end['lifetime_toxin_damage'] if end else '',resistance_cost_at_terminal=end['lifetime_resistance_cost'] if end else '')
   individuals.append(row);group_rows[(condition,level,group)].append(row)
  summaries.append(dict(condition=condition,seed=seed,level=level,rows=rows,legacy_id_rows=malformed,individuals=len(events),disappeared=complete,censored=censored,unobserved_at_end=unknown,offspring_counter_disagreements=offspring_disagreements,birth_events=sum(1 for e in events.values() if 'birth' in e)))
  if offspring_disagreements:issues.append(stem+f': offspring counters disagree for {offspring_disagreements} individuals (may reflect pre-observation births or event semantics)')
 write_csv(out/'run_quality.csv',summaries,list(summaries[0]) if summaries else ['condition','seed','level'])
 write_csv(out/'individual_fitness.csv',individuals,list(individuals[0]) if individuals else ['condition','seed','level','id'])
 groups=[]
 for (condition,level,group),rr in sorted(group_rows.items()):
  complete=[r for r in rr if r['status']=='observed_disappearance']
  groups.append(dict(condition=condition,level=level,resistance_bin=group,individuals=len(rr),observed_disappearances=len(complete),unobserved_at_end=sum(r['status']=='unobserved_at_end' for r in rr),right_censored=sum(r['status']=='right_censored' for r in rr),median_observed_children=median([r['observed_children'] for r in rr]),median_age_at_disappearance=median([r['age_at_terminal'] for r in complete if r['age_at_terminal'] is not None])))
 write_csv(out/'resistance_bin_summary.csv',groups,list(groups[0]) if groups else ['condition','level','resistance_bin'])
 for condition,level in [('B_no_toxin_cost','control'),('F_strong_cost','x1'),('F_strong_cost','x2'),('F_strong_cost','x4')]:
  n=sum(r['condition']==condition and r['level']==level for r in summaries)
  if n!=a.expected_seeds:issues.append(f'{condition}/{level}: expected {a.expected_seeds}, got {n}')
 quality=dict(phase='07.2.22.1',runs=len(summaries),individuals=len(individuals),event_counts=dict(event_count),runs_with_unobserved_survivors=sum(r['unobserved_at_end']>0 for r in summaries),issues=issues,limitations=['Legacy phase07222 files lack end-of-run survivor snapshots: missing terminal records are unknown/censored, NOT deaths.','Disappeared means removed from population; toxin-specific cause is not established.','Initial animals may have pre-observation history; observed children and survival are not lifetime-complete.','Age at disappearance is last observed age; do not interpret as exact time of death.','Group summaries are descriptive and do not adjust for age, exposure, or competing risks.','The number of offspring is not itself proof of heritable selection.'])
 (out/'quality.json').write_text(json.dumps(quality,indent=2,ensure_ascii=False),encoding='utf-8')
 lines=['# Phase 07.2.22.1 - life-history CSV recovery and fitness diagnostics','',f"Runs: {len(summaries)}, individuals: {len(individuals)}, validation issues: {len(issues)}",'',f"Observed disappearances: {event_count['disappeared']}; births: {event_count['birth']}",f"Individuals without terminal record: {sum(r['unobserved_at_end'] for r in summaries)}",'', '## Resistance-group descriptive summary','', '|Condition|Toxin|Resistance|Individuals|Disappearances|Unobserved at end|Median observed children|','|---|---|---|---:|---:|---:|---:|']
 for r in groups:lines.append(f"|{r['condition']}|{r['level']}|{r['resistance_bin']}|{r['individuals']}|{r['observed_disappearances']}|{r['unobserved_at_end']}|{r['median_observed_children']}|")
 lines+=['','## Interpretation','These are descriptive associations, not exposure-adjusted survival rates or proof of toxin-resistance evolution.','Legacy files omit survivors at tick 50000, so absent terminal events must not be counted as deaths.','Review quality.json before drawing conclusions. Future runs record explicit censored survivors.']
 (out/'REPORT.md').write_text('\n'.join(lines)+'\n',encoding='utf-8')
 print(json.dumps({'runs':len(summaries),'individuals':len(individuals),'issues':len(issues),'out':str(out)},ensure_ascii=False))
 return 2 if issues else 0
if __name__=='__main__':raise SystemExit(main())
