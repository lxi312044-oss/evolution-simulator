import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { initSync, Simulation } from "../web/src/pkg/sim_wasm.js";

const read = path => readFile(new URL(path, import.meta.url));
const loadConfig = async name => JSON.parse(await read("../config/" + name + ".json"));
const newWorld = (config, Type = Simulation) => new Type(JSON.stringify(config));
const green = [78, 203, 112];
const rgb = values => values.map(v => Math.round(v * 255));
initSync({ module: await read("../web/src/pkg/sim_wasm_bg.wasm") });

const standard = await loadConfig("standard-toxin");
const profile = structuredClone(standard);
Object.assign(profile.plants, {
  color_inheritance_enabled: true, color_mutation_probability: 1, color_mutation_scale: 0.25,
});
Object.assign(profile.toxin, {
  enabled: true, plant_variation_enabled: true, initial_toxic_plant_fraction: 0,
  plant_toxic_mutation_probability: 0.5,
});
profile.herbivores.color_learning_enabled = true;
const observed = newWorld(profile);
const control = new Simulation(observed.effective_config_json());
const snapshot = () => JSON.parse(observed.plant_color_snapshot_json());
assert.ok(snapshot().every(p => rgb(p.slice(4)).every((v, i) => v === green[i])));
for (let i = 0; i < 10; ++i) {
  observed.step(100);
  control.step(100);
  const positions = snapshot();
  assert.ok(observed.plant_count() + observed.herbivore_count() + observed.carnivore_count() <= profile.world.max_living_entities);
  const ids = observed.map_entity_ids();
  positions.forEach((p, n) => {
    const detail = JSON.parse(observed.inspect_entity_json(ids[2*n], ids[2*n+1]));
    assert.deepEqual(rgb(p.slice(4)), rgb(detail.rgb));
  });
}
assert.deepEqual(observed.audit_state(false), control.audit_state(false));
assert.ok(snapshot().some(p => rgb(p.slice(4)).some((v, i) => v !== green[i])), "expected seed offspring with a changed color");
const ids = observed.map_entity_ids();
const plantCount = observed.plant_count();
const learnedColors = Array.from({ length: observed.herbivore_count() }, (_, index) => {
  const offset = (plantCount + index) * 2;
  return JSON.parse(observed.inspect_entity_json(ids[offset], ids[offset+1])).memory ?? [];
}).flat();
assert.ok(learnedColors.some(entry => rgb(entry.rgb).some((v, i) => v !== green[i])),
  "expected a herbivore to remember a changed plant color after toxin damage");
observed.free();
control.free();

if (process.argv[2]) {
  const dir = resolve(process.argv[2]);
  const previous = await import(pathToFileURL(resolve(dir, "sim_wasm.js")).href);
  previous.initSync({ module: await readFile(resolve(dir, "sim_wasm_bg.wasm")) });
  const profiles = [
    standard, structuredClone(standard), await loadConfig("stage4-inherited-plant-colors"),
    await loadConfig("stage4-inherited-predator-candidate"),
  ];
  profiles[1].toxin.experimental_plant_toxicity_colors = true;
  profiles[1].toxin.plant_variation_enabled = true;
  for (const [i, config] of profiles.entries()) {
    const before = newWorld(config, previous.Simulation);
    const after = newWorld(config);
    assert.deepEqual(JSON.parse(after.effective_config_json()), JSON.parse(before.effective_config_json()));
    if (i < 2) {
      before.enable_standard_germination();
      after.enable_standard_germination();
    }
    for (let tick = 0; tick <= 1000; ++tick) {
      assert.deepEqual(after.audit_state(false), before.audit_state(false), "profile " + i + ", tick " + tick);
      if (tick < 1000) {
        before.step(1);
        after.step(1);
      }
    }
    before.free();
    after.free();
  }
}
console.log("Plant RGB: seed variation, inspector/map agreement, JSON and observation parity.");
