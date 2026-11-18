"""Analyze the Issue #11 40x40 field comparison.

Plan and its pre-run revision (criteria fixed before the run):
https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6970763006
https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6970805102

Usage:
  python tools/analyze_stage4_field40.py <40x40-dir> <20x20-round1-dir> <20x20-round2-dir>
"""
import json
import math
import sys
from pathlib import Path

SEEDS = range(90001, 90009)
JUDGE_TICK = 20000
ARMS = ["f40_base", "f40_decay", "f40_v42_rest", "f40_v525_rest"]
# 20x20 runs with the same settings (round 1 / round 2 of the review).
REFERENCES = {"f40_base": ("1", "s32"), "f40_decay": ("1", "s32_decay"),
              "f40_v42_rest": ("2", "v42_rest"), "f40_v525_rest": ("2", "v525_rest")}
CHANGE_ORDER = {"f40_base": 0, "f40_decay": 1, "f40_v42_rest": 2, "f40_v525_rest": 3}


def rows_of(path):
    record = json.loads(Path(path).read_text())
    return record, record["rows"]


def extinct_by(rows, column, tick):
    return any(r[column] == 0 for r in rows if r[0] <= tick)


def judge(rows):
    at = next(r for r in rows if r[0] == JUDGE_TICK)
    return {
        "carnivore_extinct": extinct_by(rows, 1, JUDGE_TICK),
        "herbivore_extinct": extinct_by(rows, 2, JUDGE_TICK),
        "coexist": all(at[i] > 0 for i in (1, 2, 3)),
    }


def herbivore_peaks(rows, half_window=20, rise=0.3):
    h = [r[2] for r in rows]
    peaks, trough = [], h[0]
    for i in range(len(h)):
        lo, hi = max(0, i - half_window), min(len(h), i + half_window + 1)
        if h[i] == max(h[lo:hi]) and h[i] > 0 and h[i] >= (1 + rise) * trough:
            if not peaks or i - peaks[-1] > half_window:
                peaks.append(i)
                trough = h[i]
        trough = min(trough, h[i])
    return [rows[i][0] for i in peaks]


def lagged_correlation(rows, max_lag_rows=50):
    h = [r[2] for r in rows]
    c = [r[1] for r in rows]
    best = (None, None)
    for lag in range(0, max_lag_rows + 1):
        x, y = h[: len(h) - lag], c[lag:]
        if len(x) < 20:
            break
        mx, my = sum(x) / len(x), sum(y) / len(y)
        sx = math.sqrt(sum((v - mx) ** 2 for v in x))
        sy = math.sqrt(sum((v - my) ** 2 for v in y))
        if sx == 0 or sy == 0:
            continue
        r = sum((a - mx) * (b - my) for a, b in zip(x, y)) / (sx * sy)
        if best[0] is None or r > best[0]:
            best = (r, lag * 100)
    return {"max_correlation": best[0], "lag_ticks": best[1]}


def summarize(runs):
    judged = [judge(rows) for _, rows in runs]
    return {
        "carnivore_extinct_count": sum(j["carnivore_extinct"] for j in judged),
        "herbivore_extinct_count": sum(j["herbivore_extinct"] for j in judged),
        "coexist_count": sum(j["coexist"] for j in judged),
    }


def main():
    f40_dir, round1_dir, round2_dir = sys.argv[1:4]
    result = {}
    for arm in ARMS:
        runs = [rows_of(Path(f40_dir) / f"{arm}_s{s}.json") for s in SEEDS]
        for record, rows in runs:
            if record["last_tick"] < JUDGE_TICK or rows[-1][0] != record["last_tick"]:
                raise SystemExit(f"incomplete run: {arm} {record['seed']}")
        s = summarize(runs)
        s["criteria"] = {
            "carnivore_extinct_le_2": s["carnivore_extinct_count"] <= 2,
            "herbivore_extinct_le_2": s["herbivore_extinct_count"] <= 2,
            "coexist_ge_6": s["coexist_count"] >= 6,
        }
        s["candidate"] = all(s["criteria"].values())
        s["last_tick"] = {r["seed"]: r["last_tick"] for r, _ in runs}
        s["carnivore_extinct_tick"] = {r["seed"]: next((x[0] for x in rows if x[1] == 0), None) for r, rows in runs}
        s["herbivore_extinct_tick"] = {r["seed"]: next((x[0] for x in rows if x[2] == 0), None) for r, rows in runs}
        cycles = {}
        for record, rows in runs:
            if record["last_tick"] == 40000 and all(rows[-1][i] > 0 for i in (1, 2, 3)):
                kills = sum(r[4][5] for r in rows)
                deaths = sum(r[4][11] + r[4][12] for r in rows)
                cycles[record["seed"]] = {
                    "herbivore_peaks": herbivore_peaks(rows),
                    **lagged_correlation(rows),
                    "predation_share_of_herbivore_deaths": kills / deaths if deaths else None,
                    "carnivore_min_max": [min(r[1] for r in rows), max(r[1] for r in rows)],
                    "herbivore_min_max": [min(r[2] for r in rows), max(r[2] for r in rows)],
                }
        s["coexisting_to_40000"] = cycles
        round_, ref_arm = REFERENCES[arm]
        ref_dir = round1_dir if round_ == "1" else round2_dir
        s["reference_20x20"] = {"arm": ref_arm, **summarize([rows_of(Path(ref_dir) / f"{ref_arm}_s{s_}.json") for s_ in SEEDS])}
        result[arm] = s
    candidates = [a for a in ARMS if result[a]["candidate"]]
    result["candidates"] = candidates
    result["selected"] = min(candidates, key=lambda a: (-result[a]["coexist_count"], CHANGE_ORDER[a])) if candidates else None
    print(json.dumps(result, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
