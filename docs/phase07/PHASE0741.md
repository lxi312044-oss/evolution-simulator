# Phase 07.4.1 — Heritable toxic / non-toxic plants

## Scope
- Opt-in binary plant toxin production phenotype; default is fully backward-compatible.
- Independent deterministic hash for founder traits and seed mutation, no extra simulation RNG consumption.
- Feeding exposure and plant synthesis energy cost only for toxic plants.
- No changes to toxin kinetics, herbivore resistance, predator behavior or plant color.

## Validation
- `cargo test --workspace`
- `powershell.exe -ExecutionPolicy Bypass -File .\tools\verify_phase0734.ps1` (standard baseline only)
- `cargo test -p sim-core --test plant_toxin0741`
- Web `wasm-pack build` and `npm run build` and browser smoke test.
- Experiment config `config/phase0741-mixed-plants.json` is separate from baseline.
