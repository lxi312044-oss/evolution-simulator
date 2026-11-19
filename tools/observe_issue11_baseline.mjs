import assert from "node:assert/strict";
import fs from "node:fs";
import { resolve } from "node:path";
import { createRequire } from "node:module";

const directory = process.argv[2];
const { Simulation } = createRequire(import.meta.url)(
  resolve(directory, "pkg/sim_wasm.js")
);
const read = name => JSON.parse(fs.readFileSync(directory + "/" + name, "utf8"));
const config = JSON.stringify(read("input.json").config);
const baseline = read("wasm.json");
const observed = new Simulation(config), control = new Simulation(config);
observed.enable_standard_germination();
control.enable_standard_germination();
const records = new Map(), details = [];
let deaths = [];
const started = performance.now();
let maxPopulation = 0;
for (const expected of baseline) {
  const tick = expected.tick;
  if (performance.now() - started > 230000) throw Error("time limit at " + tick);
  if (tick) {
    const batch = JSON.parse(observed.carnivore_baseline_batch(100));
    if (tick <= 1000) control.step(100);
    for (const e of batch.events) {
      if (e.phase !== "predator_reproduction") continue;
      const id = e.id.join(":");
      if (!records.has(id)) records.set(id, {
        id, parent: e.parent, birth: e.parent ? e.tick : 0, firstMeal: null,
        gates: {}, episodes: [], pending: null, recent: [],
      });
      const r = records.get(id);
      const ate = e.bite_n > 0 || e.bite_e > 0;
      if (ate && r.firstMeal === null) r.firstMeal = e.tick;
      if (e.resumed) {
        assert.equal(r.pending, null);
        r.pending = {start: e.tick, meal: null};
      }
      if (r.pending) {
        if (ate && r.pending.meal === null) r.pending.meal = e.tick;
        const outcome = e.fullness <= 0.25 ? "reached25" : e.feeding ? null : "stopped";
        if (outcome) {
          r.episodes.push({...r.pending, end: e.tick, outcome});
          r.pending = null;
        }
      }
      const flags = e.fail.map(Number).join("");
      const gates = [e.food,e.trend,e.stomach,e.sustainable].map(x => x === null ? "-" : Number(x)).join("");
      const key = flags + "/" + gates;
      r.gates[key] = (r.gates[key] ?? 0) + 1;
      maxPopulation = Math.max(maxPopulation, e.population);
      assert.ok(e.population <= 5000);
      if (e.tick <= r.birth + 200) details.push(e);
      r.recent[e.tick % 201] = e;
    }
    for (const d of batch.deaths.slice(deaths.length)) {
      const r = records.get(d.id_index + ":" + d.id_generation);
      details.push(...r.recent.filter(e => e.tick > r.birth + 200 && e.tick >= d.tick - 200));
      r.recent = [];
      if (r.pending) {
        r.episodes.push({...r.pending, end: d.tick, outcome:"death"});
        r.pending = null;
      }
    }
    deaths = batch.deaths;
  }
  if (tick <= 1000) assert.deepEqual(observed.audit_state(false), control.audit_state(false), "state or RNG at " + tick);
  const actual = {
    plants: observed.plant_count(), herbivores: observed.herbivore_count(), carnivores: observed.carnivore_count(),
    carnivore_births: Number(observed.carnivore_births()), carnivore_deaths: Number(observed.carnivore_deaths()),
    carnivore_hunts: Number(observed.carnivore_hunts()),
    ...JSON.parse(observed.carnivore_debug_json()), ...JSON.parse(observed.diagnostics_json()),
  };
  assert.ok(actual.plants + actual.herbivores + actual.carnivores <= 5000);
  for (const [k,v] of Object.entries(actual)) {
    if (typeof v === "number" && typeof expected[k] === "number")
      assert.ok(Math.abs(v-expected[k]) <= 1e-7, tick + " " + k);
  }
}
assert.equal(records.size, 38);
assert.equal(deaths.length, records.size);
for (const r of records.values()) assert.equal(r.pending, null);
const payload = JSON.stringify({
  records: [...records.values()].map(({recent, pending, ...r}) => r),
  deaths, details, maxPopulation, seconds:(performance.now()-started)/1000,
});
assert.ok(Buffer.byteLength(payload) <= 28 * 1024 * 1024);
fs.writeFileSync(directory + "/individual.json", payload, {flag:"wx"});
console.log("Baseline complete", records.size, deaths.length, maxPopulation, Buffer.byteLength(payload));
