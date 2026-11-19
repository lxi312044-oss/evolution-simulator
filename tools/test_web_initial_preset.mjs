import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { greenFounderPreset } from "../web/src/initial-preset.ts";
import { initSync, Simulation } from "../web/src/pkg/sim_wasm.js";

const read = path => readFile(new URL(path, import.meta.url));
initSync({ module: await read("../web/src/pkg/sim_wasm_bg.wasm") });
const standard = JSON.parse(await read("../config/standard-toxin.json"));
const original = structuredClone(standard);
const config = greenFounderPreset(standard);
assert.deepEqual(standard, original);
const expected = structuredClone(original);
Object.assign(expected.toxin, {
  enabled: true, plant_variation_enabled: true, initial_toxic_plant_fraction: 0,
  plant_toxic_mutation_probability: 0.05, experimental_plant_toxicity_colors: false,
  experimental_inherited_plant_colors: false,
});
Object.assign(expected.plants, {
  color_inheritance_enabled: true, color_mutation_probability: 0.2, color_mutation_scale: 0.25,
});
expected.herbivores.color_learning_enabled = true;
expected.carnivores.speed = 5.25;
assert.deepEqual(config, expected);

const sim = new Simulation(JSON.stringify(config));
const previousPreset = structuredClone(config);
previousPreset.carnivores.speed = 4.2;
const old = new Simulation(JSON.stringify(previousPreset));
const repeat = new Simulation(sim.effective_config_json());
assert.equal(sim.herbivore_color_snapshot_json(), old.herbivore_color_snapshot_json());
assert.equal(sim.carnivore_snapshot_json(), old.carnivore_snapshot_json());
const readPlants = () => {
  const ids = sim.map_entity_ids();
  return Array.from({ length: sim.plant_count() }, (_, i) =>
    JSON.parse(sim.inspect_entity_json(ids[2*i], ids[2*i+1])));
};
const green = [78, 203, 112];
const isGreen = p => p.rgb.every((v, i) => Math.round(v * 255) === green[i]);
const founders = readPlants();
assert.equal(founders.length, 300);
assert.ok(founders.every(p => !p.toxic && isGreen(p)));
assert.equal(JSON.parse(sim.effective_config_json()).herbivores.color_learning_enabled, true);
for (const world of [sim, old, repeat]) {
  world.enable_standard_germination();
  const initialN = world.total_nutrients();
  const initialE = world.total_energy();
  world.step(1000);
  assert.ok(Math.abs(world.total_nutrients() - initialN) < 1e-6);
  const energyError = world.total_energy() - initialE - world.sunlight_in() + world.heat_out();
  assert.ok(Math.abs(energyError) < 1e-3, String(energyError));
}
assert.deepEqual(sim.audit_state(false), repeat.audit_state(false));
for (const [name, world] of [["before", old], ["after", sim]]) {
  const d = JSON.parse(world.carnivore_debug_json());
  const h = JSON.parse(world.diagnostics_json());
  console.log(name, {
    distance: d.predator_movement_distance, movementE: d.energy_movement_spent,
    chaseMovementE: d.p41_chase_movement_energy, attacks: Number(world.carnivore_hunts()),
    huntAttempts: d.hunt_attempts, herbivoreMovementE: h.herbivore_movement_e,
    counts: [world.plant_count(), world.herbivore_count(), world.carnivore_count()],
  });
}
repeat.free();
const plants = readPlants();
assert.ok(plants.some(p => p.toxic));
assert.ok(plants.some(p => !isGreen(p)));
console.log("Web initial preset: 300 non-toxic green founders; both traits vary by Tick 1000.");
sim.free();
old.free();
