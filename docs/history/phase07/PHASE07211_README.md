# Phase 07.2.1.1 — resistance genealogy and lifetime reproductive success

Run `run_phase07211.ps1 -Ticks 1000 -SeedCount 1 -Force` followed by full run.

New `*.genealogy.csv` contains actual parent/child EntityIds, parent/child resistance, mutation **attempt** flag, and death/final snapshot offspring counts. `censored` means alive at end (not completed lifetime). `founder` means initial individual. Entity IDs include generation to avoid collisions.

This is observational instrumentation only: no changes to random draws, toxin, survival or reproduction rules. For natural selection analysis compare resistance-binned offspring counts among *dead* individuals, and separately report right-censored living individuals; control for age and exposure. Mutation attempted may leave unchanged gene due to clamp.

Web: removed obsolete `movement_strategy` field from snapshot object to resolve TS2339; no ecology change. `wasm-pack` warnings about package metadata/license are nonfatal.
