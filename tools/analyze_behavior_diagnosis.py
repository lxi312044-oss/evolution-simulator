"""Extract and hash the bounded replay's genealogy, behavior and RNG evidence."""
import argparse
import json
from pathlib import Path
from run_metadata import sha256


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    args = parser.parse_args()
    root = args.root
    verified = {}
    for directory in ['window2300', 'watch1063', 'predator-window', 'lineage-window']:
        p = root / directory
        quality = json.loads((p / 'quality.json').read_text())
        assert quality['passed'] and quality['exact_replays'] == 2
        for name, digest in quality['result_sha256'].items():
            assert sha256(p / name) == digest, f'changed replay: {p / name}'
        verified[directory] = sha256(p / 'quality.json')
    evidence = dict(verified_quality_sha256=verified, levels={})
    rng = []
    for level in ['low', 'high']:
        data = json.loads((root / 'lineage-window' / f'{level}.json').read_text())
        rows = data['trace']
        focal = data['result']['focal_id']
        def select(phase, tick=None, entity=None):
            return [x for x in rows if x['phase'] == phase
                    and (tick is None or x['tick'] == tick)
                    and (entity is None or x.get('id') == entity)]
        lineage, child = [], [2423, 2]
        while child != focal:
            event = next(x for x in rows if x['phase'] == 'birth' and x['child'] == child)
            lineage.append(event)
            child = event['parent']
            assert len(lineage) < 10, 'invalid genealogy'
        terminal = select('terminal', entity=[2423, 2])
        assert len(terminal) == 1
        draws = select('animal_rng_advance', tick=2328, entity=[1063, 1])
        focus = select('behavior', tick=2348, entity=focal)
        assert len(focus) == 1 and focus[0]['target'] is None and focus[0]['threat'] is None
        assert focus[0]['actual_step'] == focus[0]['desired_distance']
        evidence['levels'][level] = dict(lineage=lineage, descendant_terminal=terminal,
            predators_2323=[x for x in select('predator_behavior', 2323)
                            if x['id'] in [[2567, 1], [4460, 0]]],
            other_herbivore_2324=select('behavior', 2324, [1063, 1]),
            other_herbivore_rng_2328=draws, focal_2348=focus,
            focal_outcome=data['result']['outcome'])
        rng.append({x['tick']: x['rng'] for x in select('rng_end_tick')})
    evidence['first_end_tick_rng_difference'] = next(t for t in range(1, 3001)
                                                     if rng[0][t] != rng[1][t])
    evidence['rng_equal_again_at_2347'] = rng[0][2347] == rng[1][2347]
    evidence['scope'] = ('One seed. The exact focal exploration branch and a descendant/predator/'
                         'other-herbivore RNG route are observed. RNG states can resynchronize; '
                         'all intervening plant and animal branches are not exhaustively attributed. '
                         'This is not evidence of a positive average reproductive effect.')
    output = root / 'mechanism_evidence.json'
    if output.exists():
        raise ValueError('Preserve existing analysis output')
    output.write_text(json.dumps(evidence, indent=2))
    quality = dict(passed=True, exact_replays=8, unique_conditions=2, independent_seeds=1,
                   input_quality_sha256=verified, analysis_sha256=sha256(__file__),
                   evidence_sha256=sha256(output))
    (root / 'analysis_quality.json').write_text(json.dumps(quality, indent=2))
    print(json.dumps(quality))


if __name__ == '__main__':
    main()
