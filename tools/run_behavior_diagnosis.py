"""Issue #6 bounded behavior/RNG replay; never modifies ecology or searches seeds."""
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
import time
from run_metadata import record, sha256, source_hashes
from run_resistance_pilot import digest_json

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--start', type=int, default=2300)
    parser.add_argument('--end', type=int, default=2350)
    parser.add_argument('--watch', type=int, nargs=2)
    args = parser.parse_args()
    if not 1 <= args.start <= args.end <= 3000 or args.end - args.start > 100:
        raise ValueError('window must be within horizon and at most 101 ticks')
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    reference = ROOT / 'results/resistance-pilot-20261107/pilot'
    quality = json.loads((reference / 'quality.json').read_text())
    assert quality['passed']
    binary = ROOT / 'target/release/examples/resistance_focal_pilot.exe'
    protocol = dict(status='running', started_at=datetime.now(timezone.utc).isoformat(),
                    source_sha256=source_hashes(ROOT), binary_sha256=sha256(binary),
                    runner_sha256=sha256(__file__), window=[args.start, args.end],
                    seed=23451, ticks=3000, interval=100, git_commit=None, watch=args.watch,
                    reference_quality_sha256=sha256(reference / 'quality.json'))
    (out / 'protocol.json').write_text(json.dumps(protocol, indent=2))
    started = time.monotonic()
    hashes, rows = {}, []
    try:
        for level, resistance in [('low', '0.2'), ('high', '0.8')]:
            name = f'seed23451_A_type0_{level}_on'
            old_path = reference / f'{name}.result.json'
            assert sha256(old_path) == quality['input_sha256'][old_path.name]
            config = reference / 'seed23451.input.json'
            meta = out / f'{level}.metadata.json'
            record('begin', ROOT, config, binary, meta, 3000, 100, 'resistance_focal_pilot')
            command = [str(binary), str(config), 'A', '0', resistance, '0.8', '3000', '100',
                       '--behavior-trace', str(args.start), str(args.end)]
            if args.watch: command += [str(x) for x in args.watch]
            data = json.loads(subprocess.run(command, cwd=ROOT, capture_output=True,
                                            text=True, check=True, timeout=120).stdout)
            trace = data.pop('trace')
            data['initial_background_sha256'] = digest_json(data.pop('initial_background'))
            data['case'] = [23451, 'A', 0, level, 'on']
            assert data == json.loads(old_path.read_text()), 'changed trajectory'
            keep = [x for x in trace if x['phase'] in
                    ['birth', 'terminal', 'behavior', 'predator_behavior', 'animal_rng_advance', 'rng_post_herbivores', 'rng_end_tick']]
            assert len([x for x in keep if x['phase'] == 'rng_end_tick']) == 3000
            assert len([x for x in keep if x['phase'] == 'behavior' and x['id']==data['focal_id']]) == args.end-args.start+1
            path = out / f'{level}.json'
            path.write_text(json.dumps(dict(result=data, trace=keep), separators=(',', ':')))
            hashes[path.name] = sha256(path)
            rows.append(keep)
            record('complete', ROOT, config, binary, meta, 3000, 100, 'resistance_focal_pilot')
        low, high = [{x['tick']: x for x in r if x['phase'] == 'rng_end_tick'} for r in rows]
        first_rng = next((t for t in range(1, 3001) if low[t]['rng'] != high[t]['rng']), None)
        a, b = [{x['tick']: x for x in r if x['phase'] == 'behavior' and x['id']==data['focal_id']} for r in rows]
        differences = dict()
        for field in a[args.start]:
            differences[field] = next((t for t in sorted(a) if a[t][field] != b[t][field]), None)
        summary = dict(first_end_tick_rng_difference=first_rng,
                       first_behavior_field_difference=differences,
                       selected=[dict(tick=t, low=a[t], high=b[t])
                                 for t in [args.start, 2347, 2348, args.end] if t in a])
        (out / 'summary.json').write_text(json.dumps(summary, indent=2))
        hashes['summary.json'] = sha256(out / 'summary.json')
        assert source_hashes(ROOT) == protocol['source_sha256']
        assert sha256(binary) == protocol['binary_sha256']
        assert sha256(__file__) == protocol['runner_sha256']
        assert time.monotonic()-started < 600
        assert sum(p.stat().st_size for p in out.iterdir()) < 15_000_000
        protocol.update(status='completed', completed_at=datetime.now(timezone.utc).isoformat())
        (out / 'protocol.json').write_text(json.dumps(protocol, indent=2))
        (out / 'quality.json').write_text(json.dumps(dict(passed=True, exact_replays=2,
            source_manifest_sha256=digest_json(protocol['source_sha256']),
            result_sha256=hashes, elapsed_seconds=time.monotonic()-started), indent=2))
        print(json.dumps(dict(passed=True, first_rng_difference=first_rng, differences=differences)))
    except Exception as error:
        (out / 'quality.json').write_text(json.dumps(dict(passed=False, error=str(error))))
        raise


if __name__ == '__main__':
    main()
