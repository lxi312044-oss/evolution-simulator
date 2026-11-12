import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';

const [directory, scenario, mode, output] = process.argv.slice(2);
const start = performance.now();
const paths = {
    single: 'results/issue26-resistance-toxin-pilot-20261110/inputs/80624103-1.json',
    multi: 'results/phase0759-terminal-20261106/runs/seed12345_equal_on.json',
};
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const stage2 = 'results/stage2-color-learning-20261111';
const input = readFileSync(paths[scenario]);
const inputHash = sha(input);
assert.equal(inputHash, JSON.parse(readFileSync(stage2 + '/manifest.json')).hashes[paths[scenario].replaceAll("/", String.fromCharCode(92))]);
const checkpoints = JSON.parse(readFileSync(stage2 + '/' + scenario + '-off-1.json')).checkpoints;

async function load(label) {
    const path = resolve(directory, label);
    const wasm = readFileSync(resolve(path, 'sim_wasm_bg.wasm'));
    const module = await import(pathToFileURL(resolve(path, 'sim_wasm.js')));
    module.initSync({module: wasm});
    return {Simulation: module.Simulation, hash: sha(wasm)};
}

const legacy = mode === 'compat';
const small = mode === 'smoke';
assert(legacy || small || mode === 'observation');
const current = await load('current');
const baseline = legacy ? await load('baseline') : current;
const cfg = JSON.parse(input);
if (!legacy) cfg.animal_toxin = {enabled: true, color_learning_enabled: true};
if (small) cfg.initial_population = {plants: 8, herbivores: 4, carnivores: 2};
const ticks = small ? 20 : 10000;
const worlds = [new baseline.Simulation(JSON.stringify(cfg)), new current.Simulation(JSON.stringify(cfg))];
if (legacy) worlds.push(new current.Simulation(JSON.stringify(cfg)));
if (scenario === 'single') worlds.forEach(world => world.enable_standard_germination());
if (!legacy) worlds[1].enable_lineage_observation(BigInt(ticks));

const fields = ['tick', 'config', 'germination', 'allocator', 'resources', 'ledger', 'land', 'plants',
    'herbivores', 'carnivores', 'grid', 'rng', 'birth-death counts', 'diagnostics', 'carnivore debug',
    'prey trend', 'plant color memory', 'focal controls', 'behavior trace', 'health trace',
    'energy trace', 'foraging trace', 'color events', 'carnivore life events', 'animal toxins', 'herbivore color recovery'];
const view = bytes => Buffer.from(bytes.buffer, bytes.byteOffset, bytes.byteLength);
const stepMs = worlds.map(() => 0);
let auditMs = 0;
function compare(tick) {
    const left = view(worlds[0].audit_state(legacy));
    for (let i = 1; i < worlds.length; i++) {
        const right = view(worlds[i].audit_state(legacy));
        if (left.equals(right)) continue;
        const differences = [];
        for (const section of [...fields.keys(), 30]) {
            const a = view(worlds[0].audit_state(legacy, section));
            const b = view(worlds[i].audit_state(legacy, section));
            if (a.equals(b)) continue;
            let offset = 0;
            while (offset < Math.min(a.length, b.length) && a[offset] === b[offset]) offset++;
            differences.push({section: fields[section] ?? 'focal intervention marker', offset,
                leftLength: a.length, rightLength: b.length,
                left: a.subarray(offset, offset + 32).toString('hex'),
                right: b.subarray(offset, offset + 32).toString('hex')});
        }
        writeFileSync(output + '.diff.json', JSON.stringify({tick, world: i, differences}, null, 2), {flag: 'wx'});
        throw Error('First mismatch at Tick ' + tick);
    }
    if (small) assert(left.equals(view(worlds[0].audit_state(legacy))));
}

for (let tick = 0; tick <= ticks; tick++) {
    if (tick) worlds.forEach((world, i) => {
        const before = performance.now();
        world.step(1);
        stepMs[i] += performance.now() - before;
    });
    const before = performance.now();
    compare(tick);
    if (legacy && tick % 1000 === 0) {
        const expected = checkpoints[tick / 1000];
        for (const world of worlds) assert.equal(sha(world.observation_state_json()), expected.state);
    }
    if (!legacy && tick % 100 === 0 && tick < ticks) worlds[1].take_lineage_events_json();
    auditMs += performance.now() - before;
}
if (!legacy) {
    worlds[1].finish_lineage_observation();
    const events = JSON.parse(worlds[1].take_lineage_events_json());
    assert(events.some(event => event.kind === 'complete'));
    compare(ticks);
}
worlds.forEach(world => world.free());
const seconds = (performance.now() - start) / 1000;
const rssMiB = process.resourceUsage().maxRSS / 1024;
const result = {scenario, mode, ticks, comparedBoundaries: ticks + 1, inputHash,
    baselineWasm: baseline.hash, currentWasm: current.hash, stepSeconds: stepMs.map(ms => ms / 1000),
    comparisonSeconds: auditMs / 1000, seconds, rssMiB, withinBudget: seconds <= 120 && rssMiB <= 512};
writeFileSync(output, JSON.stringify(result, null, 2) + '\n', {flag: 'wx'});
console.log(JSON.stringify(result));
assert(result.withinBudget);
