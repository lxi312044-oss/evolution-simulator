"""Analyze whether herbivore speed is a disadvantage without predators (Issue #11).

Plan and criteria (fixed before the run):
https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6974147996

Usage: python tools/analyze_stage4_herbivore_speed_cost.py <result-dir>

Judgement uses the `nopred` arm: speed declines from tick 0 to the end in >=12/16 seeds
and the fastest bin's lifetime births per dead individual are <0.9x the mean of all dead
individuals (median over seeds) -> disadvantage; declines in <=8/16 and ratio >=0.9 ->
no disadvantage; otherwise inconclusive.
"""
import json
import math
import statistics
import sys
from pathlib import Path

SEEDS = range(90001, 90017)
BINS = [(0.0, 1.75), (1.75, 2.5), (2.5, 3.25), (3.25, math.inf)]
LABELS = ["<1.75", "1.75-2.5", "2.5-3.25", ">=3.25"]


def bin_of(speed):
    return next(i for i, (lo, hi) in enumerate(BINS) if lo <= speed < hi)


def per_bin(individuals):
    out = []
    for i in range(len(BINS)):
        group = [d for d in individuals if bin_of(d[0]) == i]
        out.append({
            "n": len(group),
            "births_per_individual": statistics.mean(d[2] for d in group) if group else None,
            "age_mean": statistics.mean(d[1] for d in group) if group else None,
        })
    return out


def summarize(directory, arm):
    runs = {}
    for seed in SEEDS:
        doc = json.loads((Path(directory) / f"{arm}_s{seed}.json").read_text())
        rows = doc["rows"]
        if rows[-1][0] != doc["ticks"] or len(rows) != doc["ticks"] // 100 + 1:
            raise SystemExit(f"incomplete run: {arm} {seed}")
        last_alive = next((r for r in reversed(rows) if r[2] > 0), None)
        deaths = doc["deaths"]
        mean_births = statistics.mean(d[2] for d in deaths) if deaths else None
        bins = per_bin(deaths)
        fast = bins[-1]["births_per_individual"]
        runs[seed] = {
            "speed_start": rows[0][4],
            "speed_end": last_alive[4] if last_alive else None,
            "end_tick_used": last_alive[0] if last_alive else None,
            "faster_than_carnivore_start": rows[0][5],
            "faster_than_carnivore_end": last_alive[5] if last_alive else None,
            "herbivores_end": rows[-1][2],
            "deaths": len(deaths),
            "alive_at_end": len(doc["alive_at_end"]),
            "births_per_dead_individual": mean_births,
            "bins_all": bins,
            "bins_born": per_bin([d for d in deaths if not d[3]]),
            "fastest_bin_ratio": fast / mean_births if fast is not None and mean_births else None,
            "speed_mean_every_2000": [r[4] for r in rows if r[0] % 2000 == 0],
        }
    declined = sum(1 for r in runs.values() if r["speed_end"] is not None and r["speed_end"] < r["speed_start"])
    ratios = [r["fastest_bin_ratio"] for r in runs.values() if r["fastest_bin_ratio"] is not None]
    ratio = statistics.median(ratios) if ratios else None
    bin_ratio_medians = []
    for i in range(len(BINS)):
        v = [r["bins_all"][i]["births_per_individual"] / r["births_per_dead_individual"]
             for r in runs.values()
             if r["bins_all"][i]["births_per_individual"] is not None and r["births_per_dead_individual"]]
        bin_ratio_medians.append(statistics.median(v) if v else None)
    return {
        "seeds_speed_declined": declined,
        "fastest_bin_ratio_median": ratio,
        "births_ratio_by_bin_median": dict(zip(LABELS, bin_ratio_medians)),
        "speed_start_median": statistics.median(r["speed_start"] for r in runs.values()),
        "speed_end_median": statistics.median(r["speed_end"] for r in runs.values() if r["speed_end"] is not None),
        "faster_end_median": statistics.median(
            r["faster_than_carnivore_end"] for r in runs.values() if r["faster_than_carnivore_end"] is not None),
        "runs": runs,
    }


def judge(s):
    if s["fastest_bin_ratio_median"] is None:
        return "inconclusive"
    if s["seeds_speed_declined"] >= 12 and s["fastest_bin_ratio_median"] < 0.9:
        return "disadvantage"
    if s["seeds_speed_declined"] <= 8 and s["fastest_bin_ratio_median"] >= 0.9:
        return "no_disadvantage"
    return "inconclusive"


def main():
    directory = sys.argv[1]
    result = {arm: summarize(directory, arm) for arm in ["nopred", "pred"]}
    result["judgement_nopred"] = judge(result["nopred"])
    print(json.dumps(result, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
