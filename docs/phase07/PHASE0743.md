# Phase 07.4.3 — Paired toxin-production cost and trait-selection pilot

## Goal
Estimate how plant toxin synthesis cost changes toxic-plant frequency and trait-specific demographic rates, with toxin potency, genetic mutation, environment and initial seed held fixed within each pair. Do not interpret an increase in toxic frequency as proof of toxin defense.

## Scope and frozen model
No Rust simulation behavior, Web UI, toxin physiology, inheritance, RNG consumption, or resource accounting is changed in this phase. Reuse `plant_selection_0742` and the existing mixed-plant JSON. Add only PowerShell experiment orchestration, Python stdlib analysis, unit tests, and documentation.

## Pilot design
- Seeds: 12345, 12346, 12347 (paired within seed).
- 10,000 ticks per run; samples every 100 ticks. Nine total runs.
- Cost per toxic plant per tick: zero `0`, standard `0.002`, high `0.02` (all other config fields identical within seed).
- This is a *pilot* to check direction and magnitude, not a formal significance test.
- CSV birth counts include spontaneous germination, so birth rates are population-level descriptive rates, not parent-offspring fitness.

## Metrics
- Final toxic fraction and change since tick 0.
- Paired difference of final toxic fraction vs zero-cost run for each seed.
- Trait-specific births, age deaths, consumed deaths per **plant-tick** of exposure (trapezoid approximation from sampled population sizes).
- Total toxin synthesis energy cost and extinction/undefined-denominator flags.
- Aggregate mean and range of paired effects; count seeds with positive/negative effects. No p-values for three seeds.

## Run
From project root:

```powershell
powershell.exe -ExecutionPolicy Bypass -File ".\tools\run_phase0743.ps1"
```

Outputs in `results/phase0743`: nine CSVs, nine exact run-config JSONs, `REPORT.json`.

Optional rerun without baseline (only after baseline has passed):

```powershell
powershell.exe -ExecutionPolicy Bypass -File ".\tools\run_phase0743.ps1" -SkipRegression
```

## Acceptance criteria
1. `cargo test --workspace` succeeds, including `plant_selection0742`.
2. Python analysis tests succeed and input validation catches invalid counts or nonfinite values.
3. Frozen 10,000-tick Phase 07.3.1 legacy regression passes with both resource and event CSV comparisons.
4. Nine cost-sweep runs complete and analysis outputs finite, internally consistent results (or clearly flags extinction/undefined rates).
5. Existing Web build/startup remains functional (no Web code changed).
6. Interpret findings as descriptive pilot evidence only; report any seed-dependent reversals and do not assert that predator learning is active.

## Known limits and next phase
- Rates use snapshots, not exact individual lifetimes; finer interval can improve approximation.
- Spontaneous germination can confound interpreting births as genetic reproductive success.
- Changing synthesis cost changes resource dynamics; the paired comparison captures the model's combined response, not a separated mechanistic pathway.
- Predator toxin/color recognition and Batesian mimicry remain future phases.
- Continue monitoring carnivore extinction independently.
