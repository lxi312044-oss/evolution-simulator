"""Analyze the Issue #11 40x40 run with halved carnivore basal energy (f40_E, f40_noFT_E).

Criteria are fixed in the plan before the run:
https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6971981987

Usage:
  python tools/analyze_stage4_field40_cycles.py <40x40-dir> <20x20-round3-dir>
"""
import json
import sys
from pathlib import Path

from analyze_stage4_carnivore_cycles import herbivore_peaks, lagged_correlation

SEEDS = range(90001, 90009)
END, MID = 40000, 20000
ARMS = {"f40_E": "w_E", "f40_noFT_E": "w_noFT_E"}
CHANGES = {"f40_E": 1, "f40_noFT_E": 3}
KILLS, HERB_AGE, HERB_HP = 5, 11, 12


def status(rows, tick):
    """Extinction is sticky: a run stopped early after extinction stays extinct."""
    upto = [r for r in rows if r[0] <= tick]
    last = upto[-1]
    carnivore = any(r[1] == 0 for r in upto)
    herbivore = any(r[2] == 0 for r in upto)
    reached = last[0] == tick
    return {
        "carnivore_extinct": carnivore,
        "herbivore_extinct": herbivore,
        "coexist": reached and all(last[i] > 0 for i in (1, 2, 3)),
    }


def decline_predation_share(rows):
    peak = max(range(len(rows)), key=lambda i: rows[i][1])
    end = next((i for i in range(peak, len(rows)) if rows[i][1] == 0), len(rows) - 1)
    seg = rows[peak : end + 1]
    deaths = sum(r[4][HERB_AGE] + r[4][HERB_HP] for r in seg)
    return sum(r[4][KILLS] for r in seg) / deaths if deaths else None


def summarize(path_of):
    counts = {t: {"carnivore_extinct": 0, "herbivore_extinct": 0, "coexist": 0} for t in (MID, END)}
    runs, cycle_runs, coexisting = {}, 0, 0
    for seed in SEEDS:
        record = json.loads(Path(path_of(seed)).read_text())
        rows = record["rows"]
        if rows[-1][0] < MID:
            raise SystemExit(f"incomplete run: {path_of(seed)}")
        for t in (MID, END):
            for key, value in status(rows, t).items():
                counts[t][key] += value
        run = {
            "last_tick": rows[-1][0],
            "carnivore_extinct_tick": next((r[0] for r in rows if r[1] == 0), None),
            "herbivore_extinct_tick": next((r[0] for r in rows if r[2] == 0), None),
            "carnivore_peak": max(r[1] for r in rows),
            "carnivore_min_after_peak": min(r[1] for r in rows[max(range(len(rows)), key=lambda i: rows[i][1]):]),
            "herbivore_peak": max(r[2] for r in rows),
            "decline_predation_share": decline_predation_share(rows),
        }
        if status(rows, END)["coexist"]:
            coexisting += 1
            peaks = herbivore_peaks(rows)
            r, lag = lagged_correlation(rows)
            cyclic = len(peaks) >= 2 and r is not None and r >= 0.3 and 300 <= lag <= 5000
            cycle_runs += cyclic
            run.update({"herbivore_peaks": peaks, "lag_correlation": r, "lag_ticks": lag, "cyclic": cyclic})
        runs[seed] = run
    return counts, runs, cycle_runs, coexisting


def main():
    f40_dir, w_dir = sys.argv[1:3]
    result = {}
    for arm, reference in ARMS.items():
        counts, runs, cycle_runs, coexisting = summarize(lambda s: Path(f40_dir) / f"{arm}_s{s}.json")
        s = counts[END]
        survival = {
            "carnivore_extinct_le_2": s["carnivore_extinct"] <= 2,
            "herbivore_extinct_le_2": s["herbivore_extinct"] <= 2,
            "coexist_ge_6": s["coexist"] >= 6,
        }
        cycle = coexisting > 0 and cycle_runs * 2 >= coexisting
        ref_counts, ref_runs, ref_cycles, ref_coexisting = summarize(lambda s_: Path(w_dir) / f"{reference}_s{s_}.json")
        result[arm] = {
            "at_20000": counts[MID], "at_40000": s, "survival": survival,
            "coexisting_runs": coexisting, "cycle_runs": cycle_runs, "cycle": cycle,
            "candidate": all(survival.values()) and cycle, "runs": runs,
            "reference_20x20": {"arm": reference, "at_20000": ref_counts[MID], "at_40000": ref_counts[END],
                                "runs": ref_runs},
        }
    candidates = [a for a in ARMS if result[a]["candidate"]]
    result["candidates"] = candidates
    result["selected"] = (
        min(candidates, key=lambda a: (-result[a]["at_40000"]["coexist"], -result[a]["cycle_runs"], CHANGES[a]))
        if candidates else None
    )
    print(json.dumps(result, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
