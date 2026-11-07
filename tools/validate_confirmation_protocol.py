"""Validate fixed Issue #7 design and estimate budget without running simulations."""
import argparse
from datetime import datetime
import gzip
import itertools
import json
from pathlib import Path
from run_metadata import sha256

ROOT = Path(__file__).resolve().parents[1]


def validate(path):
    plan = json.loads(path.read_text())
    matrix = plan['matrix']
    seeds = plan['seeds']
    if (matrix['environments'] != ['A', 'B', 'off'] or matrix['manipulated_toxin_types'] != [0, 1]
            or matrix['focal_resistance'] != [0.2, 0.8] or matrix['defense_effectiveness'] != [0.0, 0.8]):
        raise ValueError('fixed intervention factors changed')
    if seeds != list(range(34501, 34533)) or set(seeds) & set(plan['excluded_pilot_seeds']):
        raise ValueError('confirmation seeds changed or overlap pilot')
    if sha256(ROOT / plan['base_config']) != plan['base_config_sha256']:
        raise ValueError('base config changed')
    arms = list(itertools.product(seeds, matrix['environments'], matrix['manipulated_toxin_types'],
                                  matrix['focal_resistance'], matrix['defense_effectiveness']))
    if len(arms) != 768 or len(set(arms)) != 768 or matrix['arms_per_seed'] != 24:
        raise ValueError('incomplete or duplicate matrix')
    if (plan['ticks'], plan['interval'], plan['secondary_horizons']) != (6000, 100, [3000]):
        raise ValueError('fixed horizons changed')
    if plan['execution_limits']['arms_per_batch'] * plan['execution_limits']['batches'] != len(arms):
        raise ValueError('batch mismatch')
    prior = []
    for p in itertools.chain((ROOT / 'results').rglob('protocol.json'), (ROOT / 'results').rglob('run.json')):
        data = json.loads(p.read_text())
        used = set(data.get('seeds', []))
        if 'seed' in data:
            used.add(data['seed'])
        used.update(case[0] for case in data.get('planned_cases', []))
        if used & set(seeds):
            prior.append(p.relative_to(ROOT).as_posix())
    if prior:
        raise ValueError(f'confirmation seeds already recorded in {prior}')
    return plan, arms


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    protocol = ROOT / 'config/experiments/phase0759-confirmation-v1.json'
    plan, arms = validate(protocol)
    pilot = json.loads((ROOT / 'results/resistance-pilot-20261107/pilot/protocol.json').read_text())
    elapsed = (datetime.fromisoformat(pilot['completed_at'])
               - datetime.fromisoformat(pilot['started_at'])).total_seconds()
    runtime = elapsed / 192 * len(arms) * (6000 / 3000)
    event_sizes = []
    for level in ['low', 'high']:
        data = json.loads((ROOT / f'results/behavior-diagnosis-20261107/lineage-window/{level}.json').read_text())
        births = [dict(t=x['tick'], p=x['parent'], c=x['child'], r=x['child_resistance'])
                  for x in data['trace'] if x['phase'] == 'birth']
        event_sizes.append(len(gzip.compress(json.dumps(births, separators=(',', ':')).encode(), mtime=0)))
    # 2x horizon and 4x event-volume headroom; removals/snapshots/manifests budgeted separately.
    birth_estimate = max(event_sizes) * len(arms) * 2 * 4
    report = dict(protocol_sha256=sha256(protocol), validator_sha256=sha256(__file__),
        design_validated=True, simulations_run=0, seed_units=32, arms=len(arms),
        confirmation_seeds_absent_from_existing_protocols=True,
        runtime_seconds_linear_estimate=runtime, runtime_seconds_with_2x_headroom=runtime*2,
        births_gzip_bytes_with_4x_event_headroom=birth_estimate,
        reserved_other_output_bytes=60_000_000,
        output_estimate_bytes=birth_estimate+60_000_000,
        limits=plan['execution_limits'],
        estimates_are_not_guarantees=True, preflight_gates=plan['preflight_gates'])
    if runtime*2 >= plan['execution_limits']['total_wall_seconds']:
        raise ValueError('time estimate outside budget')
    if report['output_estimate_bytes'] >= plan['execution_limits']['total_output_bytes']:
        raise ValueError('output estimate outside budget')
    args.out.mkdir(parents=True, exist_ok=False)
    (args.out / 'design_quality.json').write_text(json.dumps(report, indent=2))
    (args.out / 'planned_arms.json').write_text(json.dumps(arms, separators=(',', ':')))
    print(json.dumps(report))


if __name__ == '__main__':
    main()
