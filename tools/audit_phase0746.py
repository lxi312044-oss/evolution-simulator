#!/usr/bin/env python3
"""Phase 07.4.6 design-only static contract audit (stdlib)."""
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def require(rel, fragments):
    text=(root/rel).read_text(encoding="utf-8")
    for f in fragments:
        assert f in text, f"{rel}: missing {f!r}"
require('crates/sim-core/src/plants/mod.rs',['produces_toxin: bool'])
require('crates/sim-core/src/animals/mod.rs',['toxin_resistance:f32','toxin_load:f64','body_color:BodyColor'])
require('crates/sim-core/src/animals/carnivore.rs',['pub struct Carnivore','pub color:BodyColor'])
require('crates/sim-core/src/toxin.rs',['pub struct ToxinTypeId','pub struct ToxinProfile','pub struct ToxinBurden','pub struct ToxinResistance','pub struct ToxinProduction'])
require('crates/sim-core/src/config.rs',['plant_variation_enabled: bool','resistance_enabled: bool'])
require('docs/phase07/PHASE0746.md',['Phase 08','草食→肉食','合格条件'])
print('PASS Phase 07.4.6 design static audit (no simulation changes)')
