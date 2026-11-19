"""Fixed seed23451 replay for Issue #29; observation-only and no seed search."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
import time

from run_metadata import record, sha256, source_hashes
from run_resistance_pilot import digest_json, name_for

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    old = ROOT / 'results/resistance-pilot-20261107/pilot'
    quality = json.loads((old / 'quality.json').read_text())
    assert quality['passed']
    binary = ROOT / 'target/release/examples/resistance_focal_pilot.exe'
    cases = [(23451, env, ty, level, defense) for env, ty in [('A', 0), ('B', 1)]
             for level in ['low', 'high'] for defense in ['off', 'on']]
    protocol = dict(started_at=datetime.now(timezone.utc).isoformat(), status='running',
                    cases=cases, ticks=3000, interval=100, source_sha256=source_hashes(ROOT),
                    binary_sha256=sha256(binary), runner_sha256=sha256(__file__),
                    reference_quality_sha256=sha256(old / 'quality.json'), git_commit=None)
    (out / 'protocol.json').write_text(json.dumps(protocol, indent=2))
    started = time.monotonic()
    summaries, hashes = [], {}
    try:
        for case in cases:
            assert time.monotonic() - started < 1800, '30 minute limit'
            name = name_for(case)
            reference = old / f'{name}.result.json'
            assert sha256(reference) == quality['input_sha256'][reference.name]
            config = old / 'seed23451.input.json'
            meta_path = out / f'{name}.metadata.json'
            record('begin', ROOT, config, binary, meta_path, 3000, 100, 'resistance_focal_pilot')
            command = [str(binary), str(config), case[1], str(case[2]),
                       '0.2' if case[3] == 'low' else '0.8',
                       '0' if case[4] == 'off' else '0.8', '3000', '100', '--trace']
            result = json.loads(subprocess.run(command, cwd=ROOT, capture_output=True,
                                               text=True, check=True, timeout=120).stdout)
            trace = result.pop('trace')
            result['case'] = list(case)
            result['initial_background_sha256'] = digest_json(result.pop('initial_background'))
            assert result == json.loads(reference.read_text()), f'trajectory changed: {name}'
            gates = [x for x in trace if x['phase'] == 'reproduction_gates']
            ends = [x for x in trace if x['phase'] == 'end_tick']
            attacks = [x for x in trace if x['phase'] == 'attack']
            births, previous = [], 0
            for x in ends:
                if x['births'] > previous:
                    births.append(x['tick'])
                    previous = x['births']
            assert len(births) == result['outcome']['births']
            summary = dict(case=list(case), outcome=result['outcome'], births=births,
                           attacks=attacks, gate_failures=dict(Counter(k for x in gates
                               for k, v in x.items() if k.endswith('_ok') and not v)),
                           food_gate_failures=sum(x['phase'] == 'food_gate_failed' for x in trace),
                           min_reserve_e=min(x['reserve_e'] for x in ends),
                           zero_reserve_ticks=sum(x['reserve_e'] <= 1e-9 for x in ends))
            # Chronological phases are emitted in actual execution order.
            summary['first_hp_nonpositive'] = next((x for x in trace
                if x.get('hp', x.get('hp_after', 1)) <= 0), None)
            summaries.append(summary)
            path = out / f'{name}.trace.json'
            path.write_text(json.dumps(dict(result=result, trace=trace), separators=(',', ':')))
            hashes[path.name] = sha256(path)
            record('complete', ROOT, config, binary, meta_path, 3000, 100, 'resistance_focal_pilot')
            assert sum(p.stat().st_size for p in out.iterdir()) < 20_000_000, '20 MB limit'
            print(f'Completed {name}', flush=True)
        assert source_hashes(ROOT) == protocol['source_sha256']
        assert sha256(binary) == protocol['binary_sha256']
        assert sha256(__file__) == protocol['runner_sha256']
        protocol.update(status='completed', completed_at=datetime.now(timezone.utc).isoformat())
        (out / 'protocol.json').write_text(json.dumps(protocol, indent=2))
        (out / 'summary.json').write_text(json.dumps(summaries, indent=2))
        hashes['summary.json'] = sha256(out / 'summary.json')
        (out / 'quality.json').write_text(json.dumps(dict(passed=True, runs=8,
            exact_reference_replays=8, independent_seeds=1, result_sha256=hashes,
            source_manifest_sha256=digest_json(protocol['source_sha256']),
            elapsed_seconds=time.monotonic()-started), indent=2))
    except Exception as error:
        (out / 'quality.json').write_text(json.dumps(dict(passed=False, error=str(error))))
        raise


if __name__ == '__main__':
    main()
