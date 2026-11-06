#!/usr/bin/env python3
"""Phase 06.2.30: compare 5 seeds x 4 modes; no third-party dependencies."""
import csv
from pathlib import Path
MODES=("hunting-only","stomach-relaxed","digestion-adjusted","combined")
ROOT=Path(__file__).resolve().parent/"results"/"phase06230"
KEYS=("carnivore_births","carnivore_deaths","carnivore_hunts","carnivore_hunt_energy_gain","stomach_reproduction_blocked","reproduction_ready","reproduction_all_gates_passed_ticks")
def num(row,key):
 try:return float(row.get(key,0) or 0)
 except (ValueError,TypeError):return 0.
for mode in MODES:
 files=sorted((ROOT/mode).glob("seed-*.csv"))
 if not files:
  print(f"{mode}: CSV missing");continue
 print(f"\n{mode} ({len(files)} seeds)")
 totals={k:0. for k in KEYS};survival=[];end=[]
 for file in files:
  with file.open(encoding="utf-8-sig",newline="") as f:
   reader=csv.DictReader(f);first=next(reader,None)
   if first is None:continue
   rows=[first,*reader];last=rows[-1]
  cols=set(last)
  predator_key=next((k for k in ("carnivores","carnivore_count","carnivore_population") if k in cols),None)
  for k in KEYS:
   if k in cols:totals[k]+=num(last,k)-num(first,k)
  if predator_key:
   survival.append(int(num(last,predator_key)>0))
   end.append(max((num(r,"tick") for r in rows if num(r,predator_key)>0),default=0))
  if num(last,"tick")!=50000:print(f"  WARNING {file.name}: last tick {last.get('tick')}")
 for k,v in totals.items():
  if v:print(f"  {k}: {v:,.2f}")
 if survival:print(f"  survived: {sum(survival)}/{len(survival)}, mean last seen tick: {sum(end)/len(end):.0f}")
