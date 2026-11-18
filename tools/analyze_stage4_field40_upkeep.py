"""Analyze the Issue #11 speed upkeep comparison on the 40x40 field (f40_E settings).

Criteria are fixed in the plan before the run (8 seeds):
https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6975171446

Usage: python tools/analyze_stage4_field40_upkeep.py <result-dir> <f40-result-dir>

Runs stop after tick 20000 once carnivores or herbivores are extinct, so extinction
before tick 40000 counts as extinct at 40000.
"""
import json
import statistics
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from analyze_stage4_carnivore_cycles import herbivore_peaks, lagged_correlation  # noqa: E402

ARMS = ["f40u_ctrl", "f40u_h05"]
CANDIDATE_ARMS = ["f40u_h05"]
SEEDS = range(90001, 90009)
END, MID = 40000, 20000
CHANGES = {"f40u_h05": 1}


def status(rows, tick):
    upto = [r for r in rows if r[0] <= tick]
    last = upto[-1]
    return {
        "carnivore_extinct": any(r[1] == 0 for r in upto),
        "herbivore_extinct": any(r[2] == 0 for r in upto),
        "coexist": last[0] == tick and all(last[i] > 0 for i in (1, 2, 3)),
    }


def speed_at(rows, tick, index):
    row = next((r for r in rows if r[0] == tick), None)
    return None if row is None else row[6][index]


def summarize(directory, arm):
    counts = {t: {"carnivore_extinct": 0, "herbivore_extinct": 0, "coexist": 0} for t in (MID, END)}
    out = {"runs": {}}
    coexisting = cycle_runs = 0
    for seed in SEEDS:
        record = json.loads((Path(directory) / f"{arm}_s{seed}.json").read_text())
        rows = record["rows"]
        last = record["last_tick"]
        ext = [r for r in rows if r[1] == 0 or r[2] == 0]
        complete = last == END or (last >= MID and ext and ext[0][0] <= last)
        if not complete or len(rows) != last // 100:
            raise SystemExit(f"incomplete run: {arm} {seed}")
        for t in (MID, END):
            for key, value in status(rows, t).items():
                counts[t][key] += value
        peak_i = max(range(len(rows)), key=lambda i: rows[i][1])
        run = {
            "last_tick": last,
            "carnivore_extinct_tick": next((r[0] for r in rows if r[1] == 0), None),
            "herbivore_extinct_tick": next((r[0] for r in rows if r[2] == 0), None),
            "carnivore_peak": rows[peak_i][1],
            "carnivore_peak_tick": rows[peak_i][0],
            "carnivore_min_after_peak": min(r[1] for r in rows[peak_i:]),
            "herbivore_min_after_carnivore_peak": min(r[2] for r in rows[peak_i:]),
            "carnivore_speed_initial": record["initial_speeds"][0],
            "carnivore_speed_at_peak": rows[peak_i][6][0],
            "carnivore_speed_last_alive": next((r[6][0] for r in reversed(rows) if r[1] > 0), None),
            "herbivore_speed_at_peak": rows[peak_i][6][3],
            "faster_herbivores_at_peak": rows[peak_i][6][4],
            "faster_herbivores_last_alive": next((r[6][4] for r in reversed(rows) if r[1] > 0), None),
        }
        cext = run["carnivore_extinct_tick"]
        if cext is not None:
            at = next(r for r in rows if r[0] == cext)
            before = rows[max(0, rows.index(at) - 1)]
            alive_h = [r for r in rows if r[0] > cext and r[2] > 0]
            run["herbivore_speed_at_carnivore_extinction"] = before[6][3]
            run["herbivore_speed_last"] = alive_h[-1][6][3] if alive_h else None
            run["herbivore_speed_last_tick"] = alive_h[-1][0] if alive_h else None
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
        "carnivore_extinct_le_2": s["carnivore_extinct"] <= 2,
        "herbivore_extinct_le_2": s["herbivore_extinct"] <= 2,
        "coexist_ge_6": s["coexist"] >= 6,
    }
    out["coexisting_runs"] = coexisting
    out["cycle_runs"] = cycle_runs
    out["cycle"] = coexisting > 0 and cycle_runs * 2 >= coexisting
    out["candidate"] = arm in CANDIDATE_ARMS and all(out["survival"].values()) and out["cycle"]
    ticks = [r["carnivore_extinct_tick"] for r in out["runs"].values() if r["carnivore_extinct_tick"]]
    out["carnivore_extinct_tick_median"] = statistics.median(ticks) if ticks else None
    out["carnivore_peak_median"] = statistics.median(r["carnivore_peak"] for r in out["runs"].values())
    pairs = [(r["herbivore_speed_at_carnivore_extinction"], r["herbivore_speed_last"])
             for r in out["runs"].values() if r.get("herbivore_speed_last") is not None]
    out["herbivore_speed_after_carnivore_extinction"] = {
        "runs": len(pairs),
        "declined": sum(1 for a, b in pairs if b < a),
        "median_at_extinction": statistics.median(a for a, _ in pairs) if pairs else None,
        "median_last": statistics.median(b for _, b in pairs) if pairs else None,
    }
    for key in ["carnivore_speed_at_peak", "carnivore_speed_last_alive", "faster_herbivores_at_peak",
                "faster_herbivores_last_alive"]:
        vals = [r[key] for r in out["runs"].values() if r[key] is not None]
        out[key + "_median"] = statistics.median(vals) if vals else None
    return out


def control_matches(directory, round3):
    """f40u_ctrl must reproduce f40_E populations (prefix up to the shorter run)."""
    mismatched = []
    for seed in SEEDS:
        mine = json.loads((Path(directory) / f"f40u_ctrl_s{seed}.json").read_text())["rows"]
        ref = json.loads((Path(round3) / f"f40_E_s{seed}.json").read_text())["rows"]
        n = min(len(mine), len(ref))
        if [r[:4] for r in mine[:n]] != [r[:4] for r in ref[:n]]:
            mismatched.append(seed)
    return mismatched


def main():
    directory, round3 = sys.argv[1], sys.argv[2]
    result = {"control_mismatched_seeds": control_matches(directory, round3)}
    result.update({arm: summarize(directory, arm) for arm in ARMS})
    candidates = [a for a in CANDIDATE_ARMS if result[a]["candidate"]]
    result["candidates"] = candidates
    result["selected"] = (
        min(candidates, key=lambda a: (-result[a]["at_40000"]["coexist"], -result[a]["cycle_runs"], CHANGES[a]))
        if candidates and not result["control_mismatched_seeds"]
        else None
    )
    print(json.dumps(result, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
