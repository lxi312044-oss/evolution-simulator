"""Issue #7: gated preflight, bounded four-batch confirmation and independent log analysis."""
import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor, wait, FIRST_COMPLETED
from datetime import datetime, timezone
import gzip
import itertools
import json
import math
from pathlib import Path
import platform
import random
import statistics
import subprocess
import threading
import time
import zipfile

from confirmation_metrics import entity, lineage_metrics, interaction, primary_summary
from run_metadata import command, sha256, source_hashes
from run_resistance_pilot import digest_json, name_for
from validate_confirmation_protocol import validate as validate_design

ROOT = Path(__file__).resolve().parents[1]
PROTOCOL = ROOT / 'config/experiments/phase0759-confirmation-v1.json'
BINARY = ROOT / 'target/release/examples/resistance_confirmation.exe'
WORKERS = 4


def require(value, message):
    if not value:
        raise ValueError(message)


def save(path, data):
    path.write_text(json.dumps(data, separators=(',', ':'), allow_nan=False), encoding='utf-8')


def now():
    return datetime.now(timezone.utc).isoformat()


def software():
    files = source_hashes(ROOT)
    for path in sorted((ROOT / 'tools').glob('*')):
        if path.suffix in ['.py', '.ps1']:
            files[path.relative_to(ROOT).as_posix()] = sha256(path)
    files[PROTOCOL.relative_to(ROOT).as_posix()] = sha256(PROTOCOL)
    plan = json.loads(PROTOCOL.read_text())
    files[plan['base_config']] = sha256(ROOT / plan['base_config'])
    return dict(files=files, manifest_sha256=digest_json(files), binary_sha256=sha256(BINARY),
                protocol_sha256=sha256(PROTOCOL))


def provenance():
    return dict(git_commit=command(ROOT, 'git', 'rev-parse', 'HEAD'),
        git_status=command(ROOT, 'git', 'status', '--porcelain', '--untracked-files=all'),
        rustc=command(ROOT, 'rustc', '--version'), cargo=command(ROOT, 'cargo', '--version'),
        python=platform.python_version(), workers=WORKERS)


def archive_source(out, sig):
    with zipfile.ZipFile(out/'source-snapshot.zip', 'x', zipfile.ZIP_DEFLATED) as archive:
        for name, digest in sig['files'].items():
            require(sha256(ROOT/name)==digest, 'source changed before archiving')
            archive.write(ROOT/name, name)


def verify_checks(path, sig):
    report=json.loads(path.read_text())
    require(report['passed'] is True and report['signature']==sig, 'checks not passed for current software')
    for name in ['python','rust','clippy','wasm','fmt','diff']:
        check=report['checks'][name]
        require(check['exit_code']==0 and sha256(ROOT/check['log'])==check['log_sha256'],
                f'validation gate failed or changed: {name}')


def cases(seeds):
    return [(seed, env, ty, level, defense) for seed in seeds for env in ['A', 'B', 'off']
            for ty in [0, 1] for level in ['low', 'high'] for defense in ['off', 'on']]


def finite(value):
    if isinstance(value, float):
        require(math.isfinite(value), 'nonfinite numeric value')
    elif isinstance(value, dict):
        for item in value.values(): finite(item)
    elif isinstance(value, list):
        for item in value: finite(item)


def normalized(value):
    return json.loads(json.dumps(value, allow_nan=False))


def check_control(initial, case):
    seed, env, ty, level, defense = case
    require((initial['seed'], initial['environment'], initial['toxin_type']) == (seed, env, ty), 'case identity mismatch')
    require(initial['resistance'] == (0.20000000298023224 if level == 'low' else 0.800000011920929), 'focal trait mismatch')
    require(initial['defense'] == (0 if defense == 'off' else 0.8), 'defense mismatch')
    toxin = initial['effective_config']['toxin']
    expected = dict(enabled=env != 'off', resistance_enabled=True, herbivore_multi_toxin_enabled=True,
        plant_toxin_type_variation_enabled=True, plant_toxin_type_count=2,
        initial_type0_fraction=0 if env == 'B' else 1, plant_toxin_type_mutation_probability=0,
        resistance_mutation_probability=0, initial_resistance=0.5,
        resistance_effectiveness=0 if defense == 'off' else 0.8,
        multi_type_resistance_upkeep_multiplier=0.5, additional_type_resistance_upkeep_multiplier=1)
    require(all(toxin[k] == v for k, v in expected.items()), 'effective control configuration mismatch')
    require(initial['effective_config']['seed'] == seed, 'effective seed mismatch')
    founders = {entity(x) for x in initial['founders']}
    require(len(founders) == len(initial['founders']) and entity(initial['focal_id']) == min(founders), 'invalid founders/focal')
    require({entity(x['id']) for x in initial['founder_traits']} == founders, 'incomplete initial traits')
    for item in initial['founder_traits']:
        want = [0.5, 0.5]
        if entity(item['id']) == entity(initial['focal_id']): want[ty] = initial['resistance']
        require(item['resistance'] == want, 'unintended founder trait change')


class LogValidator:
    def __init__(self, case, ticks, interval=100):
        self.case, self.ticks, self.interval = case, ticks, interval
        self.initial = self.complete = None
        self.births, self.terminals, self.snapshots = [], [], []
        self.traits = {}
        self.depths = {}

    def accept(self, row):
        finite(row)
        require(self.complete is None, 'data after completion')
        kind = row['kind']
        if kind == 'initial':
            require(self.initial is None, 'duplicate initial record')
            check_control(row, self.case)
            require((row['ticks'], row['interval']) == (self.ticks, self.interval), 'requested horizon mismatch')
            self.initial = row
            self.traits = {entity(x['id']): x['resistance'] for x in row['founder_traits']}
            self.depths = {entity(row['focal_id']): 0}
            return
        require(self.initial is not None, 'event before initial record')
        if kind == 'birth':
            parent, child = entity(row['parent']), entity(row['child'])
            require(parent in self.traits and child not in self.traits, 'invalid parent or duplicate birth')
            require(row['resistance'] == self.traits[parent], 'mutation despite target mutation OFF')
            depth = self.depths[parent]+1 if parent in self.depths else None
            require(row['depth'] == depth, 'logged birth depth disagrees with genealogy')
            if depth is not None: self.depths[child] = depth
            self.traits[child] = row['resistance']
            self.births.append(row)
        elif kind == 'removal':
            require(entity(row['id']) in self.traits, 'unregistered removal')
            require(row['resistance'] == self.traits[entity(row['id'])], 'terminal trait mismatch')
            self.terminals.append(row)
        elif kind == 'snapshot':
            self.snapshots.append(row)
            if row['present'] is not None:
                tick = row['tick']
                metrics = lineage_metrics(self.initial['founders'], self.initial['focal_id'],
                    [x for x in self.births if x['tick'] <= tick],
                    [x for x in self.terminals if x['tick'] <= tick], row['present'], tick, row['focal_birth_counter'])
                require(normalized(metrics) == row['metrics'], 'Rust/Python endpoint mismatch')
        elif kind == 'complete':
            self.complete = row
        else:
            raise ValueError(f'unknown record kind {kind}')

    def finish(self):
        require(self.initial is not None and self.complete is not None, 'truncated run')
        initial, complete = self.initial, self.complete
        legacy = complete['legacy_result']
        require(legacy['completed_tick'] == self.ticks, 'incomplete horizon')
        wanted = list(range(self.interval, self.ticks+1, self.interval))
        if not wanted or wanted[-1] != self.ticks: wanted.append(self.ticks)
        require([x['tick'] for x in self.snapshots] == wanted, 'missing/duplicate/unordered snapshot')
        require(len(self.births) == complete['birth_event_count'], 'birth event count mismatch')
        require(len(self.terminals) == complete['removal_event_count'], 'removal event count mismatch')
        require(legacy['effective_config'] == initial['effective_config'], 'config changed within run')
        for key in ['seed','environment','toxin_type','resistance','defense','ticks','interval','focal_id','initial']:
            require(legacy[key] == initial[key], f'legacy identity mismatch: {key}')
        require(legacy['max_n_error'] <= 1e-3 and legacy['max_e_error'] <= 1e-3, 'conservation residual too large')
        require(len(self.snapshots[-1]['present']) == complete['final_population'], 'endpoint population mismatch')
        require(legacy['outcome']['births'] == self.snapshots[-1]['focal_birth_counter'], 'focal outcome counter mismatch')
        require(complete['plant_type_counts'][0 if self.case[1] == 'B' else 1] == 0, 'pure toxin type contaminated')
        parents = Counter(entity(x['parent']) for x in self.births)
        for row in self.terminals:
            require(row['births'] == parents[entity(row['id'])], 'removed individual birth counter mismatch')
        endpoints = {str(x['tick']): x['metrics'] for x in self.snapshots if x['present'] is not None}
        require(str(self.ticks) in endpoints, 'missing final endpoint')
        all_legacy = dict(legacy, initial_background_sha256=initial['initial_background_sha256'], case=list(self.case))
        # Config's defense factor deliberately differs; dynamics must coincide for toxin-OFF controls.
        dynamics = dict(legacy)
        dynamics.pop('effective_config')
        dynamics.pop('defense')
        trajectory = digest_json(dict(births=self.births, terminals=self.terminals,
            snapshots=self.snapshots, legacy=dynamics, rng=complete['final_rng']))
        return dict(case=list(self.case), ticks=self.ticks, endpoints=endpoints,
            initial_background_sha256=initial['initial_background_sha256'], trajectory_sha256=trajectory,
            births=len(self.births), removals=len(self.terminals),
            max_n_error=legacy['max_n_error'], max_e_error=legacy['max_e_error']), all_legacy


def bytes_under(root):
    return sum(p.stat().st_size for p in root.rglob('*') if p.is_file())


class Budget:
    def __init__(self, root, seconds, byte_limit, reserved_bytes=0):
        self.root, self.deadline, self.byte_limit = root, time.monotonic()+seconds, byte_limit
        self.reserved_bytes = reserved_bytes
        self.stop = threading.Event()
        self.lock = threading.Lock()
        self.processes = set()

    def check(self):
        require(not self.stop.is_set(), 'run cancelled after technical failure')
        require(time.monotonic() < self.deadline, 'total runtime limit')
        require(bytes_under(self.root)+self.reserved_bytes < self.byte_limit, 'total output limit')

    def cancel(self):
        self.stop.set()
        with self.lock:
            for p in self.processes:
                if p.poll() is None: p.kill()


def run_case(case, ticks, directory, sig, budget):
    budget.check()
    name = name_for(case)
    output = directory / f'{name}.jsonl.gz'
    result_path = directory / f'{name}.result.json'
    meta_path = directory / f'{name}.metadata.json'
    require(not any(p.exists() for p in [output, result_path, meta_path]), 'refuse overwrite')
    config = directory.parent / 'inputs' / f'seed{case[0]}.json'
    metadata = dict(status='running', started_at=now(), case=list(case), ticks=ticks, interval=100,
        config_sha256=sha256(config), software_sha256=sig['manifest_sha256'],
        binary_sha256=sig['binary_sha256'], protocol_sha256=sig['protocol_sha256'])
    save(meta_path, metadata)
    started = time.monotonic()
    cmd = [str(BINARY), str(config), case[1], str(case[2]), '0.2' if case[3]=='low' else '0.8',
           '0' if case[4]=='off' else '0.8', str(ticks), '100']
    validator = LogValidator(case, ticks)
    timed_out = threading.Event()
    proc = timer = None
    try:
        with (directory / f'{name}.stderr.log').open('x', encoding='utf-8') as errors:
            proc = subprocess.Popen(cmd, cwd=ROOT, stdout=subprocess.PIPE, stderr=errors,
                                    text=True, encoding='utf-8')
            with budget.lock: budget.processes.add(proc)
            def timeout():
                timed_out.set()
                if proc.poll() is None: proc.kill()
            timer = threading.Timer(min(120, max(0.01, budget.deadline-time.monotonic())), timeout)
            timer.start()
            with output.open('xb') as raw, gzip.GzipFile(filename='', mode='wb', fileobj=raw, mtime=0) as gz:
                for line in proc.stdout:
                    require(not budget.stop.is_set(), 'cancelled')
                    row = json.loads(line)
                    if row.get('kind') == 'initial':
                        row['initial_background_sha256'] = digest_json(row.pop('initial_background'))
                    validator.accept(row)
                    gz.write((json.dumps(row, separators=(',', ':'), allow_nan=False)+'\n').encode())
            code = proc.wait()
            require(not timed_out.is_set(), 'per-arm or total timeout')
            require(code == 0, f'simulator exit {code}; see stderr')
        summary, legacy = validator.finish()
        metadata.update(status='completed', completed_at=now(), elapsed_seconds=time.monotonic()-started,
                        log_sha256=sha256(output))
        summary.update(log_sha256=metadata['log_sha256'], elapsed_seconds=metadata['elapsed_seconds'])
        save(result_path, summary)
        metadata['result_sha256'] = sha256(result_path)
        save(meta_path, metadata)
        return summary, legacy
    except Exception as error:
        metadata.update(status='failed', failed_at=now(), error=str(error))
        save(meta_path, metadata)
        if proc is not None and proc.poll() is None: proc.kill()
        raise
    finally:
        if timer is not None: timer.cancel()
        if proc is not None:
            proc.wait()
            if proc.stdout is not None: proc.stdout.close()
            with budget.lock: budget.processes.discard(proc)


def inputs(out, seeds, base_path):
    (out / 'inputs').mkdir()
    base = json.loads(base_path.read_text(encoding='utf-8-sig'))
    for seed in seeds:
        save(out / 'inputs' / f'seed{seed}.json', dict(base, seed=seed))


def run_matrix(matrix, ticks, directory, sig, budget, reference=None):
    directory.mkdir()
    output = {}
    executor = ThreadPoolExecutor(max_workers=WORKERS)
    futures = {executor.submit(run_case, case, ticks, directory, sig, budget): case for case in matrix}
    pending = set(futures)
    try:
        while pending:
            done, pending = wait(pending, timeout=1, return_when=FIRST_COMPLETED)
            budget.check()
            for future in done:
                case = futures[future]
                summary, legacy = future.result()
                if reference is not None:
                    path = reference / f'{name_for(case)}.result.json'
                    q = json.loads((reference / 'quality.json').read_text())
                    require(sha256(path) == q['input_sha256'][path.name], 'changed regression reference')
                    require(legacy == json.loads(path.read_text()), f'golden regression mismatch: {case}')
                output[case] = summary
                print(f'{directory.name}: {len(output)}/{len(matrix)} {name_for(case)}', flush=True)
    except Exception:
        budget.cancel()
        for f in pending: f.cancel()
        raise
    finally:
        executor.shutdown(wait=True, cancel_futures=True)
    validate_pairs(output)
    require(software() == sig, 'code or binary changed during matrix')
    return output


def validate_pairs(rows):
    backgrounds = {}
    for case, row in rows.items():
        key = case[:2]
        require(backgrounds.setdefault(key, row['initial_background_sha256']) == row['initial_background_sha256'],
                'initial background mismatch')
        if case[1] == 'off' and case[4] == 'on':
            other = (case[0], case[1], case[2], case[3], 'off')
            require(other in rows and rows[other]['trajectory_sha256'] == row['trajectory_sha256'],
                    'toxin-OFF defense toggle changed dynamics')


def preflight(out, checks_path):
    plan, _ = validate_design(PROTOCOL)
    out.mkdir(parents=True, exist_ok=False)
    sig = software()
    verify_checks(checks_path, sig)
    save(out / 'software.json', dict(signature=sig, provenance=provenance()))
    (out/'checks.json').write_bytes(checks_path.read_bytes())
    archive_source(out, sig)
    inputs(out, [23451], ROOT / plan['base_config'])
    started = time.monotonic()
    budget = Budget(out, 1800, plan['execution_limits']['total_output_bytes'])
    report = dict(status='running', started_at=now(), protocol_sha256=sha256(PROTOCOL))
    save(out / 'preflight.json', report)
    try:
        old = ROOT / 'results/resistance-pilot-20261107/pilot'
        regression = run_matrix(cases([23451]), 3000, out / 'regression', sig, budget, old)
        t = time.monotonic()
        calibration = run_matrix(cases([23451]), 6000, out / 'calibration', sig, budget)
        elapsed = time.monotonic()-t
        projected_seconds = elapsed/24*768*2
        projected_bytes = bytes_under(out / 'calibration')/24*768*2 + 1_000_000
        require(projected_seconds < plan['execution_limits']['total_wall_seconds'], 'calibrated runtime outside limit')
        require(projected_bytes + bytes_under(out) < plan['execution_limits']['total_output_bytes'], 'calibrated output outside limit')
        require(bytes_under(ROOT) < plan['execution_limits']['workspace_bytes'], 'workspace limit')
        report.update(status='passed', completed_at=now(), exact_3000_replays=len(regression),
            calibrated_6000_arms=len(calibration), calibration_seconds=elapsed,
            projected_seconds_with_2x_headroom=projected_seconds,
            projected_bytes_with_2x_headroom=projected_bytes, elapsed_seconds=time.monotonic()-started,
            signature=sig, software_file_sha256=sha256(out / 'software.json'),
            evidence_sha256={p.relative_to(out).as_posix(): sha256(p)
                            for p in sorted(out.rglob('*')) if p.is_file() and p.name!='preflight.json'})
        save(out / 'preflight.json', report)
        print(json.dumps({k:v for k,v in report.items() if k not in ['signature','evidence_sha256']}), flush=True)
    except Exception as error:
        report.update(status='failed', error=str(error), failed_at=now())
        save(out / 'preflight.json', report)
        raise


def load_run(directory, case, ticks, sig):
    name = name_for(case)
    meta = json.loads((directory / f'{name}.metadata.json').read_text())
    result_path = directory / f'{name}.result.json'
    log = directory / f'{name}.jsonl.gz'
    require(meta['status']=='completed' and meta['ticks']==ticks and meta['case']==list(case), 'incomplete metadata')
    require(meta['result_sha256']==sha256(result_path) and meta['log_sha256']==sha256(log), 'modified result/log')
    require(meta['software_sha256']==sig['manifest_sha256'] and meta['binary_sha256']==sig['binary_sha256']
            and meta['protocol_sha256']==sig['protocol_sha256'], 'wrong execution version')
    require(meta['config_sha256']==sha256(directory.parent/'inputs'/f'seed{case[0]}.json'), 'input config changed')
    v = LogValidator(case, ticks)
    with gzip.open(log, 'rt', encoding='utf-8') as stream:
        for line in stream: v.accept(json.loads(line))
    summary, _ = v.finish()
    saved = json.loads(result_path.read_text())
    require(all(saved[k]==value for k,value in summary.items()), 'saved metrics do not match raw events')
    return saved


def bootstrap(vectors):
    # One resampled seed index is shared by the full metric vector.
    rng = random.Random(750907)
    keys = list(vectors[0])
    samples = {key: [] for key in keys}
    for _ in range(20000):
        selected = rng.choices(range(32), k=32)
        for key in keys: samples[key].append(sum(vectors[i][key] for i in selected)/32)
    def quantile(values, p):
        position = (len(values)-1)*p
        i = int(position)
        return values[i]+(values[min(i+1,len(values)-1)]-values[i])*(position-i)
    return {key: None if statistics.stdev([v[key] for v in vectors])==0 else
            [quantile(sorted(values), 0.025), quantile(sorted(values), 0.975)] for key,values in samples.items()}


def analyze(out):
    run = json.loads((out / 'run.json').read_text())
    require(run['status'] in ['running','completed'], 'cannot analyze a failed/incomplete execution')
    plan = json.loads(PROTOCOL.read_text())
    sig = run['signature']
    require(sha256(PROTOCOL)==sig['protocol_sha256'], 'protocol changed')
    all_cases = cases(plan['seeds'])
    rows = {case: load_run(out/'runs', case, 6000, sig) for case in all_cases}
    validate_pairs(rows)
    require(software()==sig, 'execution software changed')
    contrasts = []
    for seed, env, ty, horizon in itertools.product(plan['seeds'], ['A','B','off'], [0,1], [3000,6000]):
        arms = [rows[(seed,env,ty,level,defense)]['endpoints'][str(horizon)]
                for level,defense in [('low','on'),('high','on'),('low','off'),('high','off')]]
        item = dict(seed=seed, environment=env, toxin_type=ty, horizon=horizon)
        for metric in ['direct_births','lineage_births','deeper_births','living_descendants','present_hp_nonpositive_descendants']:
            item[metric+'_interaction'] = interaction(*arms, metric)
            item[metric+'_on_difference'] = arms[1][metric]-arms[0][metric]
        contrasts.append(item)
    selected = [x for x in contrasts if (x['environment'],x['toxin_type'],x['horizon'])==('A',0,6000)]
    primary = primary_summary([x['direct_births_interaction'] for x in selected],
                              [x['direct_births_on_difference'] for x in selected])
    metrics = ['direct_births','lineage_births','deeper_births','living_descendants','present_hp_nonpositive_descendants']
    sensitivity = bootstrap([{key:x[key+'_interaction'] for key in metrics} for x in selected])
    secondary = []
    for env,ty,h in itertools.product(['A','B','off'], [0,1], [3000,6000]):
        subset = [x for x in contrasts if (x['environment'],x['toxin_type'],x['horizon'])==(env,ty,h)]
        item = dict(environment=env,toxin_type=ty,horizon=h,independent_seed_units=32,role='exploratory')
        for metric in metrics:
            values = [x[metric+'_interaction'] for x in subset]
            stat = primary_summary(values, [x[metric+'_on_difference'] for x in subset])
            item[metric] = stat['primary']  # Never promote a secondary endpoint to a confirmation claim.
        secondary.append(item)
    analysis = out / 'analysis'
    analysis.mkdir(exist_ok=False)
    save(analysis/'seed_contrasts.json', contrasts)
    save(analysis/'primary.json', primary)
    save(analysis/'secondary.json', secondary)
    save(analysis/'bootstrap_sensitivity.json', sensitivity)
    evidence = {p.relative_to(out).as_posix(): sha256(p) for p in sorted((out/'runs').glob('*')) if p.is_file()}
    save(analysis/'quality.json', dict(passed=True, arms=768, independent_seed_units=32,
         protocol_sha256=sig['protocol_sha256'], software_sha256=sig['manifest_sha256'],
         input_sha256=evidence, secondary_not_confirmatory=True,
         no_adaptive_evolution_claim=True))
    return primary


def confirm(out, preflight_path):
    plan, _ = validate_design(PROTOCOL)
    report = json.loads((preflight_path/'preflight.json').read_text())
    require(report['status']=='passed', 'preflight not passed')
    sig = software()
    require(sig==report['signature'], 'preflight code/version does not match')
    for name,digest in report['evidence_sha256'].items():
        require(sha256(preflight_path/name)==digest, 'preflight evidence changed')
    require(bytes_under(ROOT)<plan['execution_limits']['workspace_bytes'], 'workspace limit')
    out.mkdir(parents=True, exist_ok=False)
    (out/'runs').mkdir()
    inputs(out, plan['seeds'], ROOT/plan['base_config'])
    save(out/'software.json', dict(signature=sig,provenance=provenance()))
    archive_source(out, sig)
    run = dict(status='running', started_at=now(), signature=sig, planned_cases=cases(plan['seeds']),
        preflight_sha256=sha256(preflight_path/'preflight.json'), completed_batches=0)
    save(out/'run.json', run)
    budget = Budget(out, plan['execution_limits']['total_wall_seconds'],
                    plan['execution_limits']['total_output_bytes'], bytes_under(preflight_path))
    try:
        for batch in range(4):
            matrix = cases(plan['seeds'][batch*8:(batch+1)*8])
            # Same directory across batches; each arm has a unique fixed name.
            tmp = out/f'batch{batch+1}'
            run_matrix(matrix, 6000, tmp, sig, budget)
            # Preserve generated names/contents; move only these known new files within out.
            for path in tmp.iterdir():
                destination=out/'runs'/path.name
                require(not destination.exists(), 'batch destination exists')
                path.rename(destination)
            run['completed_batches']=batch+1
            save(out/'run.json', run)
            require(bytes_under(ROOT)<plan['execution_limits']['workspace_bytes'], 'workspace limit')
        primary=analyze(out)
        budget.check()
        run.update(status='completed', completed_at=now(), primary=primary)
        save(out/'run.json', run)
        print(json.dumps(dict(status='completed',primary=primary)), flush=True)
    except Exception as error:
        budget.cancel()
        run.update(status='failed', failed_at=now(), error=str(error))
        save(out/'run.json', run)
        raise


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('stage', choices=['preflight','confirm','analyze'])
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--preflight', type=Path)
    parser.add_argument('--checks', type=Path)
    args=parser.parse_args()
    out=args.out.resolve()
    if args.stage=='preflight':
        require(args.checks is not None, '--checks required for preflight')
        preflight(out,args.checks.resolve())
    elif args.stage=='confirm':
        require(args.preflight is not None, '--preflight required')
        confirm(out,args.preflight.resolve())
    else: print(json.dumps(analyze(out)))


if __name__=='__main__':
    main()
