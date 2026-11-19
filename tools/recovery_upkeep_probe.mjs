import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

const [input, runtime, index, generation, startArg, endArg, mode] = process.argv.slice(2);
const start = Number(startArg), end = Number(endArg);
assert(Number.isSafeInteger(start) && Number.isSafeInteger(end) && 0 < start && start < end);
assert(['paid', 'exempt'].includes(mode));
const { initSync, Simulation } = await import(pathToFileURL(resolve(runtime, 'sim_wasm.js')).href);
initSync({ module: readFileSync(resolve(runtime, 'sim_wasm_bg.wasm')) });
const sim = new Simulation(readFileSync(input, 'utf8'));
sim.enable_standard_germination();
const emit = row => console.log(JSON.stringify(row));
const drain = () => {
    for (const row of JSON.parse(sim.take_lineage_events_json())) emit(row);
};
sim.enable_lineage_observation(BigInt(end));
drain();
for (let tick = 0; tick < start;) {
    const step = Math.min(100, start - tick);
    sim.step(step);
    tick += step;
    drain();
}
const state = createHash('sha256').update(sim.observation_state_json()).digest('hex');
emit({kind: 'intervention', tick: start, id: [Number(index), Number(generation)], mode, state});
sim.configure_recovery_upkeep_probe(Number(index), Number(generation), mode === 'exempt');
assert.throws(() => sim.configure_recovery_upkeep_probe(Number(index), Number(generation), true));
for (let tick = start; tick <= end; tick++) {
    if (tick !== start) sim.step(1);
    emit({kind: 'probe', ...JSON.parse(sim.take_upkeep_probe_json())});
    drain();
}
sim.finish_lineage_observation();
drain();
sim.free();
