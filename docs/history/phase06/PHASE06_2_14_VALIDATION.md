# Phase 06.2.14 — 5-seed validation

This package starts from the Phase 06.2.14 source, including herbivore-style wandering for carnivores. The 06.2.14 validation uses the same baseline ecological settings as the 06.2.13 persistent configuration, with five seeds by default (12345–12349).

Run from the project root in PowerShell:

```powershell
cargo test --workspace
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\run_phase06214.ps1" -Ticks 1000 -SeedCount 2
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\run_phase06214.ps1"
```

The output directory is `results/phase06214/`, containing `seed-*.csv`, `config-*.json`, and `summary.csv`.

For comparison against 06.2.13, compare matching seeds 12345–12349, with equal Tick count and configuration, especially extinction tick, attack counts, hunting energy, movement energy, and search counters. This is not a randomized controlled comparison against the older 06.2.13 search algorithm because the movement logic has changed.

Note: No Rust compiler was available in the artifact build environment; compilation and simulation must be verified locally.
