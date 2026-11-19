"""Issue #54 step 1: selection on herbivore body size, aggression and fear (measurement only).

Plan: https://github.com/lxi312044-oss/evolution-simulator/issues/54#issuecomment-6976267702

Usage: python tools/analyze_defense_baseline.py <result-dir>

Descriptive only (no pass/fail). For each gene, individuals that died are split into
quartiles within each run; the ratio is lifetime births per individual in the top quartile
divided by the bottom quartile (median over seeds). Gene means are compared between the
start and tick 8000 (around the end of the carnivore peak in w_E) and the end.
"""
import json
import statistics
import sys
from pathlib import Path

SEEDS = range(90001, 90017)
GENES = ["adult_max_hp", "aggression", "fear", "speed"]
MEANS = ["hp_mean", "aggression_mean", "fear_mean", "speed_mean"]


def quartile_ratio(individuals, index):
    if len(individuals) < 8:
        return None
    ranked = sorted(individuals, key=lambda d: d[index])
    q = len(ranked) // 4
    low = statistics.mean(d[5] for d in ranked[:q])
    high = statistics.mean(d[5] for d in ranked[-q:])
    return high / low if low else None


def summarize(directory, arm):
    runs = {}
    for seed in SEEDS:
        doc = json.loads((Path(directory) / f"{arm}_s{seed}.json").read_text())
        rows = doc["rows"]
        if rows[-1][0] != doc["ticks"] or len(rows) != doc["ticks"] // 100 + 1:
            raise SystemExit(f"incomplete run: {arm} {seed}")
        at = {r[0]: r for r in rows}
        threatened = sum(r[4][4] for r in rows)
        standing = sum(r[4][5] for r in rows)
        runs[seed] = {
            "means_start": at[0][4][:4],
            "means_8000": at[8000][4][:4],
            "means_end": next((r[4][:4] for r in reversed(rows) if r[2] > 0), None),
            "threatened_samples": threatened,
            "standing_fraction": standing / threatened if threatened else None,
            "deaths": len(doc["deaths"]),
            "births_ratio_top_vs_bottom_quartile": {
                g: quartile_ratio(doc["deaths"], i) for i, g in enumerate(GENES)
            },
        }
    med = lambda v: statistics.median(v) if v else None  # noqa: E731
    out = {
        "standing_fraction_median": med([r["standing_fraction"] for r in runs.values()
                                         if r["standing_fraction"] is not None]),
        "threatened_samples_median": med([r["threatened_samples"] for r in runs.values()]),
    }
    for i, g in enumerate(MEANS):
        out[g] = {
            "start": med([r["means_start"][i] for r in runs.values()]),
            "t8000": med([r["means_8000"][i] for r in runs.values() if r["means_8000"][i] is not None]),
            "end": med([r["means_end"][i] for r in runs.values() if r["means_end"] is not None]),
        }
    out["births_ratio_top_vs_bottom_quartile_median"] = {
        g: med([r["births_ratio_top_vs_bottom_quartile"][g] for r in runs.values()
                if r["births_ratio_top_vs_bottom_quartile"][g] is not None])
        for g in GENES
    }
    out["runs"] = runs
    return out


def main():
    directory = sys.argv[1]
    print(json.dumps({arm: summarize(directory, arm) for arm in ["pred", "nopred"]},
                     indent=1, ensure_ascii=False, default=lambda x: None))


if __name__ == "__main__":
    main()
