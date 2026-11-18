"""Analyze round 3 of the Issue #11 stage 4 carnivore review (reproduction brakes, cycles).

Criteria are fixed in the plan before the run:
https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6971350554

Usage: python tools/analyze_stage4_carnivore_cycles.py <result-dir>
"""
import json
import math
import sys
from pathlib import Path

ARMS = ["w_base", "w_noF", "w_noT", "w_noFT", "w_E", "w_noF_E", "w_noT_E", "w_noFT_E"]
SEEDS = range(90001, 90017)
END, MID = 40000, 20000
KILLS, HERB_AGE, HERB_HP = 5, 11, 12


def changes(arm):
    return ("noF" in arm or "noFT" in arm) + ("noT" in arm or "noFT" in arm) + arm.endswith("E")


def herbivore_peaks(rows, half_window=20, rise=0.3):
    """Ticks of herbivore peaks: max within +-2000 ticks and >=30% above the previous trough."""
    h = [r[2] for r in rows]
    peaks, trough = [], h[0]
    for i, value in enumerate(h):
        lo, hi = max(0, i - half_window), min(len(h), i + half_window + 1)
        if value > 0 and value == max(h[lo:hi]) and value >= (1 + rise) * trough:
            if not peaks or i - peaks[-1] > half_window:
                peaks.append(i)
                trough = value
        trough = min(trough, value)
    return [rows[i][0] for i in peaks]


def lagged_correlation(rows, max_lag_rows=50):
    """Best Pearson correlation of herbivores(t) with carnivores(t + lag), lag 0..5000 ticks."""
    h = [r[2] for r in rows]
    c = [r[1] for r in rows]
    best_r, best_lag = None, None
    for lag in range(max_lag_rows + 1):
        x, y = h[: len(h) - lag], c[lag:]
        mx, my = sum(x) / len(x), sum(y) / len(y)
        sx = math.sqrt(sum((v - mx) ** 2 for v in x))
        sy = math.sqrt(sum((v - my) ** 2 for v in y))
        if sx == 0 or sy == 0:
            continue
        r = sum((a - mx) * (b - my) for a, b in zip(x, y)) / (sx * sy)
        if best_r is None or r > best_r:
            best_r, best_lag = r, lag * 100
    return best_r, best_lag


def status(rows, tick):
    upto = [r for r in rows if r[0] <= tick]
    last = upto[-1]
    return {
        "carnivore_extinct": any(r[1] == 0 for r in upto),
        "herbivore_extinct": any(r[2] == 0 for r in upto),
        "coexist": all(last[i] > 0 for i in (1, 2, 3)),
    }


def summarize(directory, arm):
    out = {"runs": {}}
    counts = {t: {"carnivore_extinct": 0, "herbivore_extinct": 0, "coexist": 0} for t in (MID, END)}
    cycle_runs = coexisting = 0
    kills = herb_deaths = 0
    for seed in SEEDS:
        record = json.loads((Path(directory) / f"{arm}_s{seed}.json").read_text())
        rows = record["rows"]
        if record["ticks"] != END or len(rows) != END // 100:
            raise SystemExit(f"incomplete run: {arm} {seed}")
        for t in (MID, END):
            for key, value in status(rows, t).items():
                counts[t][key] += value
        kills += sum(r[4][KILLS] for r in rows)
        herb_deaths += sum(r[4][HERB_AGE] + r[4][HERB_HP] for r in rows)
        run = {
            "carnivore_extinct_tick": next((r[0] for r in rows if r[1] == 0), None),
            "herbivore_extinct_tick": next((r[0] for r in rows if r[2] == 0), None),
            "carnivore_peak": max(r[1] for r in rows),
            "herbivore_peak": max(r[2] for r in rows),
        }
        if status(rows, END)["coexist"]:
            coexisting += 1
            peaks = herbivore_peaks(rows)
            r, lag = lagged_correlation(rows)
            cyclic = len(peaks) >= 2 and r is not None and r >= 0.3 and 300 <= lag <= 5000
            cycle_runs += cyclic
            run.update({"herbivore_peaks": peaks, "lag_correlation": r, "lag_ticks": lag, "cyclic": cyclic})
        out["runs"][seed] = run
    s = counts[END]
    out["at_20000"] = counts[MID]
    out["at_40000"] = s
    out["survival"] = {
        "carnivore_extinct_le_4": s["carnivore_extinct"] <= 4,
        "herbivore_extinct_le_4": s["herbivore_extinct"] <= 4,
        "coexist_ge_12": s["coexist"] >= 12,
    }
    out["cycle_runs"] = cycle_runs
    out["coexisting_runs"] = coexisting
    out["cycle"] = coexisting > 0 and cycle_runs * 2 >= coexisting
    out["candidate"] = all(out["survival"].values()) and out["cycle"]
    out["predation_share_of_herbivore_deaths"] = kills / herb_deaths if herb_deaths else None
    return out


def main():
    directory = sys.argv[1]
    result = {arm: summarize(directory, arm) for arm in ARMS}
    candidates = [a for a in ARMS if result[a]["candidate"]]
    result["candidates"] = candidates
    result["selected"] = (
        min(candidates, key=lambda a: (-result[a]["at_40000"]["coexist"], -result[a]["cycle_runs"], changes(a)))
        if candidates
        else None
    )
    print(json.dumps(result, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
