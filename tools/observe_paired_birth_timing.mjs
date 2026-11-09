import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import assert from 'node:assert/strict';

const [input, runtime, casePath] = process.argv.slice(2);
const {id, resistance, end, windows, check_ticks: checkTicks} = JSON.parse(readFileSync(casePath, 'utf8'));
const {initSync, Simulation} = await import(pathToFileURL(resolve(runtime, 'sim_wasm.js')).href);
initSync({module: readFileSync(resolve(runtime, 'sim_wasm_bg.wasm'))});
const config = readFileSync(input, 'utf8');
function create() {
    const sim = new Simulation(config);
    sim.enable_standard_germination();
    sim.intervene_focal_resistance(...id, resistance);
    return sim;
}
const sim = create();
const shadow = create();
const emit = row => console.log(JSON.stringify(row));
const inWindow = tick => windows.some(([start, stop]) => start <= tick && tick <= stop);
const gatePhases = new Set(['reproduction_gates', 'capacity_blocked', 'food_gate', 'swap_remove_move']);
sim.configure_health_observation(JSON.stringify([id]), 0n, BigInt(end));
sim.enable_cohort_foraging_observation();
sim.enable_lineage_observation(BigInt(end));
emit({kind: 'focal_initial', individual: JSON.parse(sim.health_observation_status_json())[0]});
const checked = [];
function drain() {
    for (const row of JSON.parse(sim.take_lineage_events_json())) emit(row);
    for (const row of JSON.parse(sim.take_health_observation_json())) {
        if (!gatePhases.has(row.phase) || inWindow(row.tick)) emit({kind:'health', ...row});
    }
    const data = JSON.parse(sim.take_cohort_foraging_json());
    for (const row of data.energy) emit({kind:'energy', ...row});
    for (const row of data.decisions) if (inWindow(row.tick)) emit({kind:'foraging', ...row});
}
for (let tick = 0; tick <= end; tick++) {
    if (tick) {
        sim.step(1);
        shadow.step(1);
    }
    if (tick % 1000 === 0 || tick === end || checkTicks.includes(tick)) {
        assert.equal(sim.observation_state_json(), shadow.observation_state_json(),
                     'Observation changed state or RNG at ' + tick);
        checked.push(tick);
    }
    drain();
    const {focal, plants, toxic_plants} = JSON.parse(sim.take_upkeep_probe_json());
    emit({kind:'probe', tick, focal, plants, toxic_plants});
}
sim.finish_lineage_observation();
drain();
emit({kind:'shadow_checks', ticks:checked});
sim.free();
shadow.free();
