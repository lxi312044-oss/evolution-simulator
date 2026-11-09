import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

const [input, runtime] = process.argv.slice(2);
const {initSync, Simulation} = await import(pathToFileURL(resolve(runtime, 'sim_wasm.js')).href);
initSync({module: readFileSync(resolve(runtime, 'sim_wasm_bg.wasm'))});
const sim = new Simulation(readFileSync(input, 'utf8'));
sim.enable_standard_germination();
for (let tick = 0; tick <= 22000; tick += 100) {
    if (tick) sim.step(100);
    const living = JSON.parse(sim.herbivore_color_snapshot_json()).filter(row => row[3] > 0).length;
    console.log(JSON.stringify({tick, living, plants: sim.plant_count()}));
}
sim.free();
