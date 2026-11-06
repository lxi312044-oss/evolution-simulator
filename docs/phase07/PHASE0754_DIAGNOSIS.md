# Phase 07.5.4 — Multiple toxin selection pressure and extinction diagnosis

## Purpose
Identify whether the observed herbivore crashes are attributable to type-specific toxin exposure, resistance upkeep, or ecological resource/predation feedback. Do not infer causation from population curves alone.

## Scope
Observational Rust example only. No changes to simulation, RNG, or accounting. Two existing seeds, 10,000 ticks, four counterfactual scenarios each:

- `legacy_single`: one legacy toxin type, one burden.
- `mixed_single_burden`: two inherited plant toxin types, but all feeding doses go to legacy type-0 burden; only legacy resistance upkeep.
- `mixed_multi_burden`: two plant toxin types, independent herbivore burdens and resistance upkeep.
- `mixed_no_toxin`: two inherited plant toxin types, toxin physiological effects and resistance upkeep disabled. **Not a pure toxin-only intervention:** plant toxin synthesis cost is also disabled.

The comparison `mixed_single_burden` vs `mixed_multi_burden` isolates the multi-burden mechanism more closely than comparison to `legacy_single`, but differs in toxin thresholds and resistance maintenance. `mixed_no_toxin` is an ecological negative control, not proof of a single causal factor. Scenarios share seeds but diverge in RNG consumption and ecological history; treat as paired design, not identical trajectories.

## Commands

```powershell
powershell.exe -ExecutionPolicy Bypass -File ".\tools\verify_phase0754.ps1"
powershell.exe -ExecutionPolicy Bypass -File ".\tools\run_phase0754.ps1" -Ticks 10000 -Interval 100
```

## Acceptance
1. Rust workspace and frozen 10k regression pass.
2. Eight scenario CSVs generated (2 seeds × 4 scenarios), with tick=0 and tick=10000 rows.
3. Compare population minima, peak-to-trough collapse, cumulative toxin damage, resistance cost, starvation damage, feeding, plant abundance, predator hunts, and death counters.
4. Do not label `toxin_deaths_with_burden` as proven toxin-caused deaths: it measures co-occurrence of burden at death.
5. If results are inconsistent between seeds, mark selection/extinction causality **inconclusive** and plan targeted controls rather than increasing run counts blindly.

## Status
Implementation prepared; Windows Rust tests and experiment pending.
