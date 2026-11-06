# Phase 06.2.30 — carnivore stomach reproduction gate vs digestion

## Aim
Phase 06.2.29 hunting-only is the control. Compare two independent changes and their combination without altering N/E conservation or artificially adding individuals.

| Mode | reproduction_stomach_max_fraction | digestion_energy_per_tick | digestion_nutrients_per_tick |
|---|---:|---:|---:|
| hunting-only | 0.25 | 0.04 | 0.007 |
| stomach-relaxed | 0.60 | 0.04 | 0.007 |
| digestion-adjusted | 0.25 | 0.08 | 0.014 |
| combined | 0.60 | 0.08 | 0.014 |

All other parameters (including herbivore well-fed streak 60 ticks and predator hunting mode) are identical. Note that increasing digestion transfers energy/nutrients from stomach to reserve; it does not create resources.

## Run from project root (PowerShell)

```powershell
cargo test --workspace

powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\run_phase06230.ps1" -Ticks 1000 -SeedCount 1

powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\run_phase06230.ps1"

python .\analyze_phase06230.py

```

## Web

```powershell
wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg

cd web
npm install
npm run dev

```

When running wasm-pack from `web`, use `wasm-pack build ../crates/sim-wasm --target web --out-dir ../../web/src/pkg` instead. The generated `web/src/pkg` directory is not included in the source archive. The condition dropdown includes four Phase 06.2.30 JSON presets. Initial preset: hunting-only.

## Evaluate
Carnivore births, reproduction gate rejection, digestive throughput, hunting energy, starvation deaths, last surviving tick, plant/herbivore persistence, N/E conservation. All variants are experimental, not confirmed improvements. If no predator survives, prioritize energy balance diagnostics.
