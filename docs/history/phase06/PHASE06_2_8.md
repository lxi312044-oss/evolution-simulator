# Phase 06.2.8 — Hunting efficiency and post-kill satiety

This phase compares 5 modes with 10 identical seeds each. Default settings retain legacy behavior (`satiety_enabled=false`).

- baseline: phase06.2.7 settings
- movement: movement energy cost 0.008 -> 0.0072
- cooldown: attack cooldown 20 -> 17 ticks
- satiety: lethal bite triggers a full meal and 250 ticks of resting; nonlethal bites do not trigger satiety
- combined: movement, cooldown and satiety together

## Satiety mechanics

When a carnivore's attack reduces a herbivore's HP to zero, it transfers up to 10 extra energy and 1.5 nutrients from the prey's existing accounts. The carnivore then rests for 250 ticks, does not pursue or attack prey, but continues basal metabolism and may reproduce. If the prey lacks resources, only the available amount is transferred; the timer still starts. No resources are created. Normal nonlethal bites use existing mechanics.

## Verification

```powershell
cargo test --workspace
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\run_phase0628.ps1 -Ticks 1000 -SeedCount 2
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\run_phase0628.ps1
```

CSV columns added: `satiated_predator_ticks`, `kills_with_meal`, `meal_bonus_energy`, `meal_bonus_nutrients`.
Compare three-species survival, carnivore extinction times, prey survival, hunts, energy balance, and conservation error. Ten seeds is exploratory, not sufficient to guarantee long-term coexistence. Rust compilation and simulation were not run in the artifact environment.
