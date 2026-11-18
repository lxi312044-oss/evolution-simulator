"""Issue #54 step 3: herbivore counterattack and carnivore learning from injury.

Plan: https://github.com/lxi312044-oss/evolution-simulator/issues/54#issuecomment-6976559257
Change before the run (any bitten herbivore fights back):
https://github.com/lxi312044-oss/evolution-simulator/issues/54#issuecomment-6976705347

Usage: python tools/analyze_counterattack.py <result-dir> <issue11-round3-result-dir>

M1: counterattack learning happened in >=12/16 seeds (d_c2, d_c5).
M2: births ratio, top vs bottom quartile of aggression among recorded herbivores, median >=1.0.
S : carnivore extinct <=4/16, herbivore extinct <=4/16, coexist >=12/16 at tick 40000.
Mimicry signs are descriptive only.
"""
import json
import math
import statistics
import sys
from pathlib import Path

ARMS = ["d_ctrl", "d_c2", "d_c5", "d_c5_nolearn"]
SEEDS = range(90001, 90017)
END, MID = 40000, 20000
COLOR_SKIPS, KILLS = 6, 5
AGGR, BIRTHS = 1, 8


def status(rows, tick):
    upto = [r for r in rows if r[0] <= tick]
    last = upto[-1]
    return {
        "carnivore_extinct": any(r[1] == 0 for r in upto),
        "herbivore_extinct": any(r[2] == 0 for r in upto),
        "coexist": last[0] == tick and all(last[i] > 0 for i in (1, 2, 3)),
    }


def quartile_ratio(individuals, index):
    if len(individuals) < 8:
        return None
    ranked = sorted(individuals, key=lambda d: d[index])
    q = len(ranked) // 4
    low = statistics.mean(d[BIRTHS] for d in ranked[:q])
    high = statistics.mean(d[BIRTHS] for d in ranked[-q:])
    return high / low if low else None


def spearman(x, y):
    def ranks(v):
        order = sorted(range(len(v)), key=lambda i: v[i])
        r = [0.0] * len(v)
        i = 0
        while i < len(v):
            j = i
            while j + 1 < len(v) and v[order[j + 1]] == v[order[i]]:
                j += 1
            for k in range(i, j + 1):
                r[order[k]] = (i + j) / 2
            i = j + 1
        return r
    rx, ry = ranks(x), ranks(y)
    mx, my = statistics.mean(rx), statistics.mean(ry)
    sx = math.sqrt(sum((a - mx) ** 2 for a in rx))
    sy = math.sqrt(sum((b - my) ** 2 for b in ry))
    if sx == 0 or sy == 0:
        return None
    return sum((a - mx) * (b - my) for a, b in zip(rx, ry)) / (sx * sy)


def mimicry_signs(individuals):
    """Among low-aggression herbivores, is being colored like aggressive ones linked to births?"""
    if len(individuals) < 16:
        return None
    ranked = sorted(individuals, key=lambda d: d[AGGR])
    q = len(ranked) // 4
    top = ranked[-q:]
    model = [statistics.mean(d[i] for d in top) for i in (4, 5, 6)]
    low = ranked[: len(ranked) // 2]
    dist = [math.dist(model, d[4:7]) for d in low]
    births = [d[BIRTHS] for d in low]
    return spearman(dist, births)


def summarize(directory, arm):
    counts = {t: {"carnivore_extinct": 0, "herbivore_extinct": 0, "coexist": 0} for t in (MID, END)}
    runs = {}
    for seed in SEEDS:
        doc = json.loads((Path(directory) / f"{arm}_s{seed}.json").read_text())
        rows = doc["rows"]
        last = doc["last_tick"]
        ext = [r for r in rows if r[1] == 0 or r[2] == 0]
        if not (last == END or (last >= MID and ext)) or len(rows) != last // 100:
            raise SystemExit(f"incomplete run: {arm} {seed}")
        for t in (MID, END):
            for key, value in status(rows, t).items():
                counts[t][key] += value
        individuals = doc["deaths"] + doc["alive_at_end"]
        peak_i = max(range(len(rows)), key=lambda i: rows[i][1])
        runs[seed] = {
            "last_tick": last,
            "carnivore_extinct_tick": next((r[0] for r in rows if r[1] == 0), None),
            "herbivore_extinct_tick": next((r[0] for r in rows if r[2] == 0), None),
            "carnivore_peak": rows[peak_i][1],
            "counterattacks": sum(r[6][0] for r in rows),
            "counterattack_energy": sum(r[6][1] for r in rows),
            "learning_events": sum(r[6][2] for r in rows),
            "color_skips": sum(r[4][COLOR_SKIPS] for r in rows),
            "kills": sum(r[4][KILLS] for r in rows),
            "aggression_start_end": [rows[0][6][4], next((r[6][4] for r in reversed(rows) if r[2] > 0), None)],
            "aggression_births_ratio": quartile_ratio(individuals, AGGR),
            "hp_births_ratio": quartile_ratio(individuals, 0),
            "mimicry_spearman_distance_vs_births_low_aggression": mimicry_signs(individuals),
        }
    med = lambda v: statistics.median(v) if v else None  # noqa: E731
    vals = lambda k: [r[k] for r in runs.values() if r[k] is not None]  # noqa: E731
    s = counts[END]
    out = {
        "at_20000": counts[MID],
        "at_40000": s,
        "M1_seeds_with_learning": sum(1 for r in runs.values() if r["learning_events"] > 0),
        "M2_aggression_births_ratio_median": med(vals("aggression_births_ratio")),
        "hp_births_ratio_median": med(vals("hp_births_ratio")),
        "survival": {
            "carnivore_extinct_le_4": s["carnivore_extinct"] <= 4,
            "herbivore_extinct_le_4": s["herbivore_extinct"] <= 4,
            "coexist_ge_12": s["coexist"] >= 12,
        },
        "carnivore_extinct_tick_median": med(vals("carnivore_extinct_tick")),
        "carnivore_peak_median": med(vals("carnivore_peak")),
        "counterattacks_median": med(vals("counterattacks")),
        "counterattack_energy_median": med(vals("counterattack_energy")),
        "color_skips_median": med(vals("color_skips")),
        "kills_median": med(vals("kills")),
        "mimicry_spearman_median": med(vals("mimicry_spearman_distance_vs_births_low_aggression")),
        "runs": runs,
    }
    out["M1"] = arm in ("d_c2", "d_c5") and out["M1_seeds_with_learning"] >= 12
    out["M2"] = out["M2_aggression_births_ratio_median"] is not None and out["M2_aggression_births_ratio_median"] >= 1.0
    out["S"] = all(out["survival"].values())
    return out


def control_matches(directory, round3):
    bad = []
    for seed in SEEDS:
        mine = json.loads((Path(directory) / f"d_ctrl_s{seed}.json").read_text())["rows"]
        ref = json.loads((Path(round3) / f"w_E_s{seed}.json").read_text())["rows"]
        if [r[:4] for r in mine] != [r[:4] for r in ref[: len(mine)]]:
            bad.append(seed)
    return bad


def main():
    directory, round3 = sys.argv[1], sys.argv[2]
    result = {"control_mismatched_seeds": control_matches(directory, round3)}
    result.update({arm: summarize(directory, arm) for arm in ARMS})
    print(json.dumps(result, indent=1, ensure_ascii=False, default=lambda x: None))


if __name__ == "__main__":
    main()
