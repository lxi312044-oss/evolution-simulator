import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import assert from 'node:assert/strict';

const [input, runtime, mode, resistanceArg = '0.8'] = process.argv.slice(2);
const resistance = Number(resistanceArg);
assert(['paid', 'exempt'].includes(mode));
assert([0.2, 0.8].includes(resistance));
const { initSync, Simulation } = await import(pathToFileURL(resolve(runtime, 'sim_wasm.js')).href);
initSync({ module: readFileSync(resolve(runtime, 'sim_wasm_bg.wasm')) });
const sim = new Simulation(readFileSync(input, 'utf8'));
sim.enable_standard_germination();
const [focal] = JSON.parse(sim.intervention_candidates_json());
const receipt = JSON.parse(sim.intervene_focal_resistance(...focal, resistance));
assert.throws(() => sim.configure_upkeep_probe(4294967295, 0, true));
sim.configure_upkeep_probe(...focal, mode === 'exempt');
assert.throws(() => sim.configure_upkeep_probe(...focal, true));

const emit = row => process.stdout.write(JSON.stringify(row) + '\n');
const drain = () => {
    for (const row of JSON.parse(sim.take_lineage_events_json())) emit(row);
};
emit({ kind: 'probe_config', mode, receipt });
sim.enable_lineage_observation(7000n);
drain();
for (let tick = 0; tick <= 7000; tick++) {
    if (tick) sim.step(1);
    emit({ kind: 'probe', ...JSON.parse(sim.take_upkeep_probe_json()) });
    drain();
}
sim.finish_lineage_observation();
drain();
sim.free();
