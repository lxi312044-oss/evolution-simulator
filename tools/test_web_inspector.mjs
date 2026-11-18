import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { initSync, Simulation } from "../web/src/pkg/sim_wasm.js";

initSync({ module: await readFile(new URL("../web/src/pkg/sim_wasm_bg.wasm", import.meta.url)) });
const config = JSON.parse(await readFile(new URL("../config/stage4-inherited-predator-candidate.json", import.meta.url)));
const create = () => new Simulation(JSON.stringify(config));
const plantToxinOff = structuredClone(config);
plantToxinOff.toxin.enabled = false;
const off = new Simulation(JSON.stringify(plantToxinOff));
const offReloaded = new Simulation(off.effective_config_json());
assert.deepEqual(off.audit_state(false), offReloaded.audit_state(false));
const restoredConfig = JSON.parse(off.effective_config_json());
assert.equal(restoredConfig.toxin.enabled, false);
assert.equal(restoredConfig.animal_toxin.enabled, true);
assert.equal(restoredConfig.toxin.experimental_inherited_plant_colors, true);
restoredConfig.toxin.enabled = true;
const onAgain = new Simulation(JSON.stringify(restoredConfig));
const fresh = create();
assert.deepEqual(onAgain.audit_state(false), fresh.audit_state(false));
assert.deepEqual(off.map_entity_ids(), fresh.map_entity_ids());
assert.equal(off.plant_color_snapshot_json(), fresh.plant_color_snapshot_json());
plantToxinOff.initial_population.plants = 299;
assert.throws(() => new Simulation(JSON.stringify(plantToxinOff)), /initial_population.plants を10の倍数/);
for (const sim of [off, offReloaded, onAgain, fresh]) sim.free();
const control = create(), observed = create(), lineage = create();
const ids = observed.map_entity_ids();
const counts = [observed.plant_count(), observed.herbivore_count(), observed.carnivore_count()];
const offsets = [0, counts[0] * 2, (counts[0] + counts[1]) * 2];
assert.equal(ids.length, counts.reduce((a, b) => a + b, 0) * 2);
for (const [i, kind] of ["plant", "herbivore", "carnivore"].entries()) {
  const pair = Array.from(ids.slice(offsets[i], offsets[i] + 2));
  const entity = JSON.parse(observed.inspect_entity_json(...pair));
  assert.deepEqual(entity.id, pair);
  assert.equal(entity.kind, kind);
  assert.equal(observed.inspect_entity_json(pair[0], pair[1] + 1), "null");
}
assert.deepEqual(JSON.parse(observed.plant_color_snapshot_json())[0].slice(4).map(Math.fround),
  JSON.parse(observed.inspect_entity_json(ids[0], ids[1])).rgb.map(Math.fround));
const restored = new Simulation(observed.effective_config_json());
assert.deepEqual(restored.audit_state(false), observed.audit_state(false));
lineage.enable_lineage_observation(100n);
for (let i = 0; i < 100; i++) {
  observed.map_entity_ids();
  observed.plant_color_snapshot_json();
  observed.herbivore_color_snapshot_json();
  observed.carnivore_snapshot_json();
  observed.inspect_entity_json(ids[0], ids[1]);
  observed.step(1);
  lineage.step(1);
}
for (let i = 0; i < 10; i++) control.step(10);
assert.deepEqual(observed.audit_state(false), control.audit_state(false));
lineage.finish_lineage_observation();
const events = JSON.parse(lineage.take_lineage_events_json());
assert.equal(events.at(-1).kind, "complete");
assert.equal(lineage.observation_state_json(), control.observation_state_json());
let removed = false;
for (let i = 0; i < 30 && !removed; i++) {
  observed.step(100);
  removed = offsets.some(offset => observed.inspect_entity_json(ids[offset], ids[offset + 1]) === "null");
}
assert(removed, "an initially selected complete ID should eventually be removed");
for (const sim of [control, observed, lineage, restored]) sim.free();
console.log("Web WASM: all species, complete IDs/removal, JSON, lineage, full-state/RNG equality at Tick 100");
