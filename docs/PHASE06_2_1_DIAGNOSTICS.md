# Phase 06.2.1 — Predator extinction investigation and plant sprites

## Scope

- Adds **cumulative, mutually exclusive first-failed-condition counters** to predator asexual reproduction. These count evaluated predator-ticks, **not individual animals**. A predator may contribute to a counter many times.
- Adds cumulative nutrient/energy transfers during successful bite contacts, and age/energy-depletion predator deaths.
- Adds existing herbivore age/HP death counters to the CLI CSV.
- Adds 16×16 pixel-art plant sprites for seedling, young and mature growth stages. Plant RGB recoloring is supported by the renderer, but plant color genes, toxins, and color-based behavior are **not implemented**.
- No ecological parameters, feeding/reproduction rules, RNG, resource transfers, or default settings were modified.

## Run

```powershell
cargo test --workspace
cargo run --release -p sim-core --example carnivore_verification -- config/phase062-carnivores.json 50000 100 results/carnivore0621.csv
wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg
cd web
npm install
npm run build
npm run dev
```

Create `results` before CLI run if needed. Compare population and conservation columns to Phase06.2 using the same config and seed: they should match exactly except for the newly appended diagnostics.

## Interpreting reproduction counters

The order of checks is cooldown, age, body nutrients, reserve nutrients, energy, then entity capacity. Each tick records **only the first failing condition**, so a large `repro_body_n_blocked` count means body nutrients were the first failure, not necessarily the only missing resource. `repro_ready` counts successful eligibility checks. `carnivore_births` counts actual offspring.

`hunt_contacts` counts bite contacts (including multiple contacts against the same prey), not prey kills. `hunt_nutrients` and `hunt_energy` record actual transferred amounts.

## Known limitations

The previous analysis suspected that reproduction checks body nutrients while bites transfer nutrients to reserve. This build **instruments rather than changes** that logic to verify the hypothesis. If the hypothesis is confirmed, implement a separately tested body-growth/assimilation mechanism in a follow-up phase, preserving N/E conservation.

The CLI uses 100-tick sampling by default. Death counters classify predator death as age or energy depletion; when both conditions are true age takes precedence. Herbivore HP deaths include damage from multiple causes; the current engine does not yet attribute every herbivore death uniquely to predator bites versus starvation. Plant rendering is cosmetic and does not affect the simulation.
