"""Analyze the Issue #11 stage 4 carnivore settings review.

Criteria are fixed in the plans before each run:
round 1: https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6968323277
round 2: https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6968964204

Usage: python tools/analyze_stage4_carnivore_review.py <result-dir> [1|2]
"""
import json
import math
import sys
from pathlib import Path

ROUNDS = {
    "1": ["s32", "s42", "s525", "s32_decay", "s42_decay", "s525_decay"],
    "2": ["v42", "v42_sustain", "v42_rest", "v42_both", "v525", "v525_sustain", "v525_rest", "v525_both"],
}
ARMS = ROUNDS["1"]
SEEDS = range(90001, 90017)
SPEED_ORDER = {"s32": 0, "s42": 1, "s525": 2}
# Round 2 herbivore criterion: round 1 s32_decay had 2/16 herbivore extinctions (+2 allowed).
ROUND1_S32_DECAY_HERBIVORE_EXTINCT = 2


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


def first_zero(rows, column):
    return next((r[0] for r in rows if r[column] == 0), None)


def summarize(runs, arm):
    counts = runs[arm, SEEDS[0]]["row_fields"][4]
    energy = runs[arm, SEEDS[0]]["row_fields"][5]
    n = dict.fromkeys(counts, 0)
    e = dict.fromkeys(energy, 0.0)
    carnivore_ticks = 0
    carn_ext, herb_ext, plant_ext, coexist, ln_ratio = {}, {}, {}, 0, []
    for seed in SEEDS:
        record = runs[arm, seed]
        rows = record["rows"]
        carn_ext[seed] = first_zero(rows, 1)
        herb_ext[seed] = first_zero(rows, 2)
        plant_ext[seed] = first_zero(rows, 3)
        last = rows[-1]
        coexist += all(last[i] > 0 for i in (1, 2, 3))
        for row in rows:
            carnivore_ticks += row[1] * 100
            for key, value in zip(counts, row[4]):
                n[key] += value
            for key, value in zip(energy, row[5]):
                e[key] += value
        gn0, _, pn0, _ = record["initial_plants_gn_gt_pn_pt"]
        gn, _, pn, _ = record["final_plants_gn_gt_pn_pt"]
        ln_ratio.append(math.log((pn + 1) / (pn0 + 1)) - math.log((gn + 1) / (gn0 + 1)))
    deaths = n["carnivore_energy_deaths"] + n["carnivore_age_deaths"] + n["carnivore_toxin_deaths"]
    per_tick = lambda value: value / carnivore_ticks if carnivore_ticks else float("nan")
    spent = e["energy_basal_spent"] + e["energy_movement_spent"] + e["energy_reproduction_spent"]
    return {
        "carnivore_extinct": carn_ext,
        "carnivore_extinct_count": sum(t is not None for t in carn_ext.values()),
        "herbivore_extinct": herb_ext,
        "herbivore_extinct_count": sum(t is not None for t in herb_ext.values()),
        "plant_extinct_count": sum(t is not None for t in plant_ext.values()),
        "coexist_count": coexist,
        "carnivore_ticks": carnivore_ticks,
        "carnivore_death_share_energy": n["carnivore_energy_deaths"] / deaths if deaths else None,
        "carnivore_intake_per_tick": per_tick(e["stomach_energy_digested"]),
        "carnivore_spend_per_tick": per_tick(spent),
        "carnivore_movement_share_of_spend": e["energy_movement_spent"] / spent if spent else None,
        "kills_per_1000_carnivore_ticks": 1000 * per_tick(n["kills_with_meal"]),
        "color_skips": n["color_skips"],
        "learned_attacks": n["learned_attacks"],
        "learned_feeds": n["learned_feeds"],
        "carnivore_toxin_deaths": n["carnivore_toxin_deaths"],
        "mimic_ln_ratio_mean": sum(ln_ratio) / len(ln_ratio),
    }


def rank_round2(result, arm):
    mechanisms = int("sustain" in arm or "both" in arm) + int("rest" in arm or "both" in arm)
    speed = 0 if arm.startswith("v42") else 1
    return (-result[arm]["coexist_count"], mechanisms, speed)


def main():
    global ARMS
    round_ = sys.argv[2] if len(sys.argv) > 2 else "1"
    ARMS = ROUNDS[round_]
    runs = load(sys.argv[1])
    result = {arm: summarize(runs, arm) for arm in ARMS}
    candidates = []
    for arm in ARMS:
        r = result[arm]
        if round_ == "1":
            reference = "s32_decay" if arm.endswith("_decay") else "s32"
            herbivore_limit = result[reference]["herbivore_extinct_count"] + 2
        else:
            herbivore_limit = ROUND1_S32_DECAY_HERBIVORE_EXTINCT + 2
        c1 = r["carnivore_extinct_count"] <= 4
        c2 = r["herbivore_extinct_count"] <= herbivore_limit
        c3 = r["coexist_count"] >= 12
        r["criteria"] = {"carnivore_extinct_le_4": c1, "herbivore_not_worse_by_more_than_2": c2, "coexist_ge_12": c3}
        if c1 and c2 and c3:
            candidates.append(arm)
    if round_ == "1":
        def rank(arm):
            speed = SPEED_ORDER[arm.removesuffix("_decay")]
            return (-result[arm]["coexist_count"], speed, arm.endswith("_decay"))
    else:
        rank = lambda arm: rank_round2(result, arm)
    result["round"] = round_
    result["candidates"] = candidates
    result["selected"] = min(candidates, key=rank) if candidates else None
    print(json.dumps(result, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
