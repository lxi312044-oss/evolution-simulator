import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import assert from 'node:assert/strict';

const [input, runtime, casePath] = process.argv.slice(2);
const {ids, start, end, shadow: verify, foraging} = JSON.parse(readFileSync(casePath, 'utf8'));
const {initSync, Simulation} = await import(pathToFileURL(resolve(runtime, 'sim_wasm.js')).href);
initSync({module: readFileSync(resolve(runtime, 'sim_wasm_bg.wasm'))});
const config = readFileSync(input, 'utf8');
function world() {
    const sim = new Simulation(config);
    sim.enable_standard_germination();
    return sim;
}

const observed = world();
const shadow = verify ? world() : null;
const emit = row => console.log(JSON.stringify(row));
observed.configure_health_observation(JSON.stringify(ids), BigInt(start), BigInt(end));
observed.enable_lineage_observation(BigInt(end));
const drain = () => {
    for (const row of JSON.parse(observed.take_lineage_events_json())) emit(row);
    for (const row of JSON.parse(observed.take_health_observation_json())) {
        emit({kind: 'health', ...row});
    }
    if (foraging && observed.tick_count() > BigInt(start)) {
        const data = JSON.parse(observed.take_cohort_foraging_json());
        for (const row of data.decisions) emit({kind: 'foraging', ...row});
        for (const row of data.energy) emit({kind: 'energy', ...row});
    }
};

let checked = 0;
for (let tick = 0; tick <= end; tick += 100) {
    if (tick) {
        observed.step(100);
        if (shadow) shadow.step(100);
    }
    if (shadow && (tick % 1000 === 0 || tick === start || tick === end)) {
        assert.equal(observed.observation_state_json(), shadow.observation_state_json(),
                     'Observation changed state or RNG at Tick ' + tick);
        checked++;
    }
    drain();
    emit({kind: 'population', tick, ...JSON.parse(observed.living_resistance_distribution_json())});
    if (tick === start) {
        emit({kind: 'cohort', tick, individuals: JSON.parse(observed.health_observation_status_json())});
        if (foraging) observed.enable_cohort_foraging_observation();
    }
}
observed.finish_lineage_observation();
drain();
emit({kind: 'shadow_checks', count: checked});
observed.free();
if (shadow) shadow.free();
