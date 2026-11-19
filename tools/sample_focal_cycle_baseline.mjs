import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

const [input, runtime, mode] = process.argv.slice(2);
const {initSync, Simulation} = await import(pathToFileURL(resolve(runtime, 'sim_wasm.js')).href);
initSync({module: readFileSync(resolve(runtime, 'sim_wasm_bg.wasm'))});
const sim = new Simulation(readFileSync(input, 'utf8'));
sim.enable_standard_germination();
const lineage = mode === 'lineage';
const drain = () => {
    for (const row of JSON.parse(sim.take_lineage_events_json())) {
        console.log(JSON.stringify(row));
    }
};
if (lineage) sim.enable_lineage_observation(22000n);
for (let tick = 0; tick <= 22000; tick += 100) {
    if (tick) sim.step(100);
    if (lineage) {
        drain();
        console.log(JSON.stringify({kind: 'population', tick,
            ...JSON.parse(sim.living_resistance_distribution_json())}));
        continue;
    }
    const living = JSON.parse(sim.herbivore_color_snapshot_json()).filter(row => row[3] > 0).length;
    console.log(JSON.stringify({tick, living, plants: sim.plant_count()}));
}
if (lineage) {
    sim.finish_lineage_observation();
    drain();
}
sim.free();
