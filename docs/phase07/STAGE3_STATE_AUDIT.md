# Phase 3 every-tick state audit

The audit compares worlds inside one Node process using the feature-gated
`state-audit` WASM export. Each boundary from Tick 0 through Tick 10,000 is
serialized into memory with bincode 1.3.3. The comparison checks every byte
directly. Floating-point values retain their binary representation, including
signed zero. SHA-256 identifies the inputs and builds and checks the previous
Stage 2 checkpoints. The every-tick equality check itself has no hash collision
risk.

## Scope

- The OFF check uses the pinned Stage 2 source `787034ab6de01e1e9d0f585a1f767117fa3cab0d`
  with the same read-only audit export added, compared with two Stage 3 OFF
  worlds. The Stage 3 extension may be omitted from the legacy projection only
  with both flags false and all added state exactly equal to its empty default.
  Historical Stage 2 checkpoints independently anchor the instrumented baseline.
- The observation check compares Stage 3 ON worlds with and without lineage
  observation. The projection includes ordered entity and resource stores,
  full IDs and allocator state, spatial buckets, configurations, germination,
  RNG, HP, production and residual toxins, both color memories, counters,
  behavioral history, and event/trace state shared by both worlds.
- The lineage observer's own accumulation and two output-enable flags differ by
  design. Its two world-side event queues are asserted empty after each
  completed step. Those queues are consumed within the step and have no
  ecological readers. The observer completion event is checked separately.
- The export borrows the world immutably. A small fixture checks sensitivity to
  signed zero and RNG changes and compares stepped worlds with and without
  repeated audit reads. The WASM smoke test checks the observed and unobserved
  paths before the fixed 10,000-Tick audit.
- Successful runs save a single summary. A comparison failure records the first
  Tick, the differing section, first byte offset, lengths and byte excerpts.

## Reproduction

Run from the repository root with the two accepted local configuration files
and Stage 2 checkpoints available. The builder keeps scratch sources and Cargo
cache below `target` and the two audit WASM files below the requested output.

```powershell
cargo test -p sim-core --features state-audit --lib state_audit
python tools/build_stage3_state_audit.py results/stage3-every-tick-audit
node tools/verify_stage3_state_audit.mjs results/stage3-every-tick-audit multi smoke results/stage3-every-tick-audit/smoke.json
```

Run the same Node command once for each pair of scenario (`single`, `multi`)
and comparison (`compat`, `observation`), with a new result path each time.
The scenarios retain seed 80624103 on 40x40 and seed 12345 on 20x20.

The Stage 1 and Stage 3 Node/WASM budgets apply to end-to-end time including
initialization and audit. The original 120-second limit also applies here. The
summary records step time for each world, comparison time, combined process
time, and peak RSS separately. The observed world's step time includes lineage
collection. Audit byte buffers never go to disk. The prior 512 MiB RSS and
20 MiB persistent-output limits still apply.
