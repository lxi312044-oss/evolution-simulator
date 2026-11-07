"""Verify focal replay files and extract paired branch times without causal inference."""
import argparse
import json
from pathlib import Path
from run_metadata import sha256


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    args = parser.parse_args()
    root = args.root
    quality = json.loads((root / 'quality.json').read_text())
    assert quality['passed'] and quality['runs'] == 8
    for name, digest in quality['result_sha256'].items():
        assert sha256(root / name) == digest, f'changed evidence: {name}'
    pairs = []
    for env, ty in [('A', 0), ('B', 1)]:
        traces = [json.loads((root / f'seed23451_{env}_type{ty}_{level}_on.trace.json').read_text())['trace']
                  for level in ['low', 'high']]
        phases = {}
        for phase in ['end_tick', 'reproduction_gates']:
            low, high = [{x['tick']: x for x in rows if x['phase'] == phase} for rows in traces]
            common = sorted(low.keys() & high.keys())
            phases[phase] = dict(selected=[dict(tick=t, low=low[t], high=high[t])
                for t in [2347, 2348, 2493, 2494, 2515, 2535, 2715] if t in common])
            if phase == 'end_tick':
                for label, fields in [('position', ['x', 'y']), ('birth', ['births'])]:
                    phases[phase][f'first_{label}_difference'] = next((t for t in common
                        if any(low[t][f] != high[t][f] for f in fields)), None)
        pairs.append(dict(environment=env, toxin_type=ty, phases=phases))
    result = dict(schema_version=1, pairs=pairs,
                  interpretation_scope='Proximate attack and reproduction gates; upstream spatial cause unresolved')
    output = root / 'paired_analysis.json'
    if output.exists():
        raise ValueError('Analysis output exists; preserve prior evidence')
    output.write_text(json.dumps(result, indent=2))
    (root / 'analysis_quality.json').write_text(json.dumps(dict(passed=True,
        analysis_sha256=sha256(__file__), output_sha256=sha256(output),
        replay_quality_sha256=sha256(root / 'quality.json')), indent=2))
    print(json.dumps(dict(passed=True, paired_conditions=len(pairs))))


if __name__ == '__main__':
    main()
