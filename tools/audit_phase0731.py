#!/usr/bin/env python3
"""Phase 07.3.1 static baseline audit. No cargo or simulation needed."""
from pathlib import Path
import hashlib, json, sys
root=Path(__file__).resolve().parents[1]
manifest=json.loads((root/'docs/phase07/phase0731-baseline-sha256.json').read_text(encoding='utf-8'))
errors=[]
for name,expected in manifest.items():
    path=root/name
    if not path.exists():
        errors.append(f'MISSING: {name}')
    elif hashlib.sha256(path.read_bytes()).hexdigest()!=expected:
        errors.append(f'CHANGED: {name}')
checks={
 'crates/sim-core/src/world/tick.rs':['plant_energy_cost_per_tick','toxin.enabled'],
 'crates/sim-core/src/world/herbivore_tick.rs':['dose_per_nutrient','clearance_fraction_per_tick','damage_threshold','resistance_effectiveness','resistance_energy_cost_per_tick','resistance_mutation_probability'],
 'crates/sim-core/src/animals/mod.rs':['toxin_resistance','toxin_load'],
 'crates/sim-core/src/config.rs':['pub struct ToxinSettings','resistance_enabled'],
}
for name,needles in checks.items():
    data=(root/name).read_text(encoding='utf-8')
    for needle in needles:
        if needle not in data: errors.append(f'MISSING_SYMBOL: {name}::{needle}')
config=json.loads((root/'config/standard-toxin.json').read_text(encoding='utf-8'))
if config.get('toxin',{}).get('hp_damage_per_excess_unit')!=0.010:
    errors.append('STANDARD_TOXIN: hp_damage_per_excess_unit must be 0.010')
if config.get('toxin',{}).get('enabled') is not True:
    errors.append('STANDARD_TOXIN: toxin.enabled must be true')
print(f'Phase 07.3.1 static audit: {len(manifest)} baseline files, {sum(map(len,checks.values()))} symbol checks')
for error in errors: print('FAIL:',error)
if errors: sys.exit(1)
print('PASS: static source baseline and standard toxin configuration')
print('NOTE: this does not replace cargo test, runtime regression or browser verification')
