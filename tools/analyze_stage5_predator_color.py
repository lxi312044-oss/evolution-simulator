"""Issue #39 primary RGB analysis on audited, paired founder summaries."""
import argparse
import csv
import json
from collections import defaultdict

import numpy as np
from scipy.stats import t

SEEDS = tuple(range(1574032001, 1574032129))
FOUNDERS = 80
MARGIN = 0.25
ALPHA = 0.05
RCOND = 1e-12
CHANNELS = ("r8", "g8", "b8")


def analyze(rows, expected_seeds=SEEDS):
    by_seed = defaultdict(list)
    expected = set(expected_seeds)
    for row in rows:
        seed = int(row["seed"])
        if seed not in expected:
            raise ValueError(f"unexpected seed {seed}")
        by_seed[seed].append(row)

    slopes = {}
    invalid = {}
    for seed in expected_seeds:
        group = by_seed[seed]
        if len(group) != FOUNDERS:
            invalid[seed] = f"expected {FOUNDERS} paired founders, got {len(group)}"
            continue
        ids = {(int(row["index"]), int(row["generation"])) for row in group}
        if len(ids) != FOUNDERS or any(index < 0 or generation < 0 for index, generation in ids):
            invalid[seed] = "duplicate or invalid complete founder ID"
            continue
        rgb = np.array([[int(row[c]) for c in CHANNELS] for row in group])
        births = np.array([[int(row[c]) for c in ("on_births", "off_births")] for row in group])
        if np.any((rgb < 0) | (rgb > 255)) or np.any(births < 0):
            invalid[seed] = "invalid RGB or descendant count"
            continue
        color = rgb / 255.0
        design = np.column_stack((np.ones(FOUNDERS), (color - color.mean(axis=0)) / 0.1))
        coefficients, _, rank, _ = np.linalg.lstsq(
            design, births[:, 0] - births[:, 1], rcond=RCOND
        )
        if rank != 4:
            invalid[seed] = "rank-deficient RGB regression"
            continue
        if not np.all(np.isfinite(coefficients)):
            invalid[seed] = "nonfinite RGB regression"
            continue
        slopes[seed] = coefficients[1:].tolist()

    intervals = {}
    critical = None
    n = len(slopes)
    decision = "uncertain"
    if n >= 2:
        values = np.array(list(slopes.values()))
        means = values.mean(axis=0)
        deviations = values.std(axis=0, ddof=1)
        critical = float(t.ppf(1 - ALPHA / (2 * len(CHANNELS)), df=n - 1))
        half_width = critical * deviations / np.sqrt(n)
        lower, upper = means - half_width, means + half_width
        intervals = {
            name: {"mean": float(mu), "seed_sd": float(sd), "lower": float(lo), "upper": float(hi)}
            for name, mu, sd, lo, hi in zip(CHANNELS, means, deviations, lower, upper)
        }
        if not invalid:
            if np.any((lower > MARGIN) | (upper < -MARGIN)):
                decision = "supported"
            elif np.all((lower >= -MARGIN) & (upper <= MARGIN)):
                decision = "not_supported"
    return {
        "decision": decision,
        "intervals_descriptive_only": bool(invalid),
        "valid_seeds": n,
        "expected_seeds": len(expected_seeds),
        "invalid_seeds": invalid,
        "critical_t": critical,
        "intervals": intervals,
        "seed_slopes_rgb": slopes,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paired_csv", help="Audited CSV: seed,index,generation,r8,g8,b8,on_births,off_births")
    args = parser.parse_args()
    with open(args.paired_csv, encoding="utf-8-sig", newline="") as stream:
        result = analyze(csv.DictReader(stream))
    print(json.dumps(result, ensure_ascii=False, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
