"""Analyze the Issue #2 (ECO-001) color memory comparison.

Criteria are fixed in the plan before the run:
https://github.com/lxi312044-oss/evolution-simulator/issues/2#issuecomment-6967338200

Usage: python tools/analyze_eco001_color_memory.py results/eco001-color-memory-update
"""
import json
import math
import sys
from pathlib import Path

ARMS = ["on", "off", "relief", "decay", "both"]
SEEDS = range(90001, 90017)
BIRTHS, AGE, HP, HP_ZERO_E, TOX_EATEN, NON_EATEN, TOX_AGE, NON_AGE, FEEDING = range(9)


def load(directory):
    runs = {}
    for arm in ARMS:
        for seed in SEEDS:
            path = Path(directory) / f"{arm}_s{seed}.json"
            if not path.exists():
                raise SystemExit(f"missing run: {path}")
            record = json.loads(path.read_text())
            if record["ticks"] != 20000 or len(record["rows"]) != 200:
                raise SystemExit(f"incomplete run: {path}")
            runs[arm, seed] = record
    return runs


def extinct_tick(record):
    return next((row[0] for row in record["rows"] if row[1] == 0), None)


def summarize(runs, arm):
    seeds = list(SEEDS)
    extinct = {s: extinct_tick(runs[arm, s]) for s in seeds}
    totals = [0] * 9
    herbivore_ticks = 0
    for s in seeds:
        for tick, herbivores, _carnivores, _plants, delta in runs[arm, s]["rows"]:
            totals = [a + b for a, b in zip(totals, delta)]
            herbivore_ticks += herbivores * 100
    snaps = [x for s in seeds for x in runs[arm, s]["snapshots"] if 4000 < x["tick"] <= 12000]
    animals = sum(x["herbivores"] for x in snaps)
    ln_ratio = []
    for s in seeds:
        gn0, _, pn0, _ = runs[arm, s]["initial_plants_gn_gt_pn_pt"]
        gn, _, pn, _ = runs[arm, s]["snapshots"][-1]["plants"]
        ln_ratio.append(math.log((pn + 1) / (pn0 + 1)) - math.log((gn + 1) / (gn0 + 1)))
    eaten = lambda e, a: totals[e] / (totals[e] + totals[a]) if totals[e] + totals[a] else float("nan")
    return {
        "extinct": extinct,
        "extinct_count": sum(t is not None for t in extinct.values()),
        "births_per_herbivore_1000_ticks": 1000 * totals[BIRTHS] / herbivore_ticks,
        "reserve_ready_share_4000_12000": animals and sum(x["reserve_ready"] for x in snaps) / animals,
        "risky_both_share_4000_12000": animals and sum(x["risky_both"] for x in snaps) / animals,
        "eaten_fraction_toxic": eaten(TOX_EATEN, TOX_AGE),
        "eaten_fraction_nontoxic": eaten(NON_EATEN, NON_AGE),
        "mimic_ln_ratio_mean": sum(ln_ratio) / len(ln_ratio),
    }


def main():
    runs = load(sys.argv[1])
    result = {arm: summarize(runs, arm) for arm in ARMS}
    base = result["on"]["extinct"]
    for arm in ["relief", "decay", "both"]:
        r = result[arm]
        rescued = sum(base[s] is not None and r["extinct"][s] is None for s in SEEDS)
        lost = sum(base[s] is None and r["extinct"][s] is not None for s in SEEDS)
        improved = r["extinct_count"] <= result["on"]["extinct_count"] - 4 and rescued > lost
        learning_kept = r["eaten_fraction_toxic"] < r["eaten_fraction_nontoxic"]
        r["paired_rescued_vs_on"] = rescued
        r["paired_lost_vs_on"] = lost
        r["criterion_improved"] = improved
        r["criterion_learning_kept"] = learning_kept
        r["candidate"] = improved and learning_kept
    print(json.dumps(result, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
