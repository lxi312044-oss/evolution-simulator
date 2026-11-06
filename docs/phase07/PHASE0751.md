# Phase 07.5.1 — Plant toxin-type gene and mutation

## Objective
Introduce a separate inherited toxin-type gene to plants without changing existing toxic/non-toxic status or ecology RNG consumption.

## Scope
- Opt-in `toxin.plant_toxin_type_variation_enabled` (default false)
- `toxin.plant_toxin_type_count` (1..16, default 1)
- `toxin.plant_toxin_type_mutation_probability` (0..1, default 0)
- Founders and spontaneous germination draw a type using deterministic hashing.
- Offspring inherit parental type, with mutation selecting a *different* type.
- Non-toxic plants retain their toxin-type genotype, but still do not synthesize toxin.
- This phase does NOT change exposure chemistry: feeding continues to use the legacy toxin type 0.

## Acceptance
- Inheritance, mutation, founder determinism and validation unit tests pass.
- `cargo test --workspace` passes.
- Phase 07.3.3 frozen regression passes with feature disabled.
- The new gene does not consume ecology RNG or alter resource balances.

## Deferred
- Actual toxin-type-specific plant exposure and herbivore resistance integration.
- Per-type population diagnostics and long-run selection experiments.
- Web build verification.
