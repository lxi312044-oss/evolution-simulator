# Phase 07.4.2 — Plant toxin-trait selection diagnostics

## Purpose
Observe whether toxic and non-toxic plant frequencies change through births, deaths, and toxin synthesis costs, without assuming either trait is always fitter.

## Scope
- Observational cumulative counters: trait-specific births, age deaths, consumed deaths, and mutation-direction births.
- CSV example `plant_selection_0742` recording population, trait frequency, demography, synthesis energy cost, and herbivore count.
- No change to toxin physics, inheritance, RNG sequence, resource accounting, or Web UI.
- Spontaneous germination is counted as a founder birth by phenotype; not as a parent-offspring inheritance event.

## Caveats
- Cumulative birth and death counts are not generation-normalized fitness estimates.
- Death by consumption is not necessarily caused by absence of toxin; predator learning is not yet implemented.
- Cost is aggregated across toxic plants, not measured per lineage; controlled comparisons are needed for causal attribution.
- An extinct trait has an undefined within-trait reproduction rate; do not divide by zero.

## Acceptance criteria
1. `cargo test --workspace` succeeds including trait-demography consistency tests.
2. Phase 07.3 baseline remains unchanged in the old configuration.
3. A mixed-trait run produces valid CSV, nonnegative counts and frequencies within [0,1] while plants exist.
4. Web build and startup work; no new UI fields are required.

## Run
From the project root: `powershell.exe -ExecutionPolicy Bypass -File .\tools\run_phase0742.ps1`

Next: compare controlled toxin synthesis costs across identical seeds and quantify per-trait birth/death rates with explicit population-time denominators before claiming selection.
