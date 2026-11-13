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
assert.deepEqual(config, expected);

const sim = new Simulation(JSON.stringify(config));
const old = new Simulation(JSON.stringify(original));
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
sim.enable_standard_germination();
sim.step(1000);
const plants = readPlants();
assert.ok(plants.some(p => p.toxic));
assert.ok(plants.some(p => !isGreen(p)));
console.log("Web initial preset: 300 non-toxic green founders; both traits vary by Tick 1000.");
sim.free();
old.free();
