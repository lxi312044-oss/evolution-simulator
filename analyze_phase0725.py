"""Phase 07.2.5 descriptive analysis of individual-life records.

Works with Python 3.10+ (including 3.14); no inference of causality.
"""
import argparse
import csv
from collections import defaultdict
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--root', default='results/phase0725')
parser.add_argument('--out', default='results/phase0725/analysis')
args = parser.parse_args()
root = Path(args.root)
out = Path(args.out)
out.mkdir(parents=True, exist_ok=True)

required = {
    'resistance', 'successful_births', 'reproduction_age_reached',
    'cumulative_toxin_dose', 'cumulative_toxin_damage',
    'cumulative_prevented_damage', 'cumulative_resistance_cost', 'status'
}
fields = ['run', 'resistance_bin', 'n', 'mean_births', 'mature_fraction',
          'mean_dose', 'mean_damage', 'mean_prevented', 'mean_cost',
          'censored_fraction']
groups = defaultdict(list)
files = sorted(root.glob('*.individual-life.csv'))
if not files:
    parser.error(f'No *.individual-life.csv files found in {root.resolve()}. Run run_phase0725.ps1 first.')

for path in files:
    with path.open(encoding='utf-8-sig', newline='') as handle:
        reader = csv.DictReader(handle)
        missing = required - set(reader.fieldnames or [])
        if missing:
            parser.error(f'{path}: missing CSV columns: {", ".join(sorted(missing))}')
        for line_no, row in enumerate(reader, start=2):
            try:
                resistance = float(row['resistance'])
                group = 'low' if resistance < 0.49 else 'high' if resistance > 0.51 else 'mid'
                # Validate all fields once so errors identify the source record.
                numeric = [float(row[name]) for name in (
                    'successful_births', 'cumulative_toxin_dose',
                    'cumulative_toxin_damage', 'cumulative_prevented_damage',
                    'cumulative_resistance_cost')]
                if row['status'] not in ('dead', 'censored'):
                    raise ValueError(f'unknown status {row["status"]!r}')
                if row['reproduction_age_reached'].lower() not in ('true', 'false'):
                    raise ValueError('invalid reproduction_age_reached')
            except (ValueError, TypeError, KeyError) as exc:
                parser.error(f'{path}:{line_no}: invalid individual record: {exc}')
            groups[(path.name.removesuffix('.individual-life.csv'), group)].append((row, numeric))

summary = []
for (run, group), records in sorted(groups.items()):
    n = len(records)
    if n == 0:
        continue
    # sum / n is robust across Python versions and avoids generator consumption issues.
    summary.append({
        'run': run, 'resistance_bin': group, 'n': n,
        'mean_births': sum(values[0] for _, values in records) / n,
        'mature_fraction': sum(row['reproduction_age_reached'].lower() == 'true' for row, _ in records) / n,
        'mean_dose': sum(values[1] for _, values in records) / n,
        'mean_damage': sum(values[2] for _, values in records) / n,
        'mean_prevented': sum(values[3] for _, values in records) / n,
        'mean_cost': sum(values[4] for _, values in records) / n,
        'censored_fraction': sum(row['status'] == 'censored' for row, _ in records) / n,
    })

with (out / 'individual_summary.csv').open('w', newline='', encoding='utf-8') as handle:
    writer = csv.DictWriter(handle, fieldnames=fields)
    writer.writeheader()
    writer.writerows(summary)

n_runs = len({run for run, _ in groups})
n_records = sum(len(records) for records in groups.values())
(out / 'REPORT.md').write_text(
    '# Phase 07.2.5 individual observations\n\n'
    f'Runs: {n_runs}; individual end records: {n_records}; groups: {len(summary)}\n\n'
    'These are descriptive comparisons, not evidence of causality. '
    'Deaths and final survivors are distinguished; censoring, birth cohort, '
    'exposure and relatedness must be controlled before selection is inferred.\n',
    encoding='utf-8'
)
print(f'Runs: {n_runs} records: {n_records} groups: {len(summary)} output: {out}')
