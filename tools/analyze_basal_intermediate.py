"""Issue #11: intermediate carnivore basal energy, 40x40, 8 seeds.

Plan: https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6983577403

Usage: python tools/analyze_basal_intermediate.py <result-dir> <f40c-result-dir> <f40-result-dir>

S: carnivore extinct <=2/8, herbivore extinct <=2/8, coexist >=6/8 at tick 40000, plus the
Issue 11 cycle criterion. Candidates: m_0096, m_0120 meeting S and the cycle.
m_0072 must match f40_E (f40c results) and m_0144 must match f40_v525_rest (f40 results).
"""
import json
import statistics
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from analyze_counterattack import END, KILLS, MID, status  # noqa: E402
from analyze_stage4_carnivore_cycles import herbivore_peaks, lagged_correlation  # noqa: E402

ARMS = ["m_0072", "m_0096", "m_0120", "m_0144"]
CANDIDATES = ["m_0096", "m_0120"]
SEEDS = range(90001, 90009)
DIGESTED, MOVEMENT = 0, 2
SPEED = 6


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
        peak_i = max(range(len(rows)), key=lambda i: rows[i][1])
        cext = next((r[0] for r in rows if r[1] == 0), None)
        alive = [r for r in rows if r[1] > 0]
        ticks = sum(r[1] for r in alive) * 100
        at = {r[0]: r for r in rows}
        run = {
            "last_tick": last,
            "carnivore_extinct_tick": cext,
            "herbivore_extinct_tick": next((r[0] for r in rows if r[2] == 0), None),
            "carnivore_peak": rows[peak_i][1],
            "carnivore_min_after_peak": min(r[1] for r in rows[peak_i:]),
            "herbivore_min_after_carnivore_peak": min(r[2] for r in rows[peak_i:]),
            "kills": sum(r[4][KILLS] for r in rows),
            "carnivore_digested_per_tick": sum(r[5][DIGESTED] for r in alive) / ticks if ticks else None,
            "carnivore_movement_per_tick": sum(r[5][MOVEMENT] for r in alive) / ticks if ticks else None,
            "herbivore_speed_every_4000": [at[t][6][SPEED] if t in at else None for t in range(4000, 40001, 4000)],
        }
        if status(rows, END)["coexist"]:
            peaks = herbivore_peaks(rows)
            r, lag = lagged_correlation(rows)
            run.update({"herbivore_peaks": peaks, "lag_correlation": r, "lag_ticks": lag,
                        "cyclic": len(peaks) >= 2 and r is not None and r >= 0.3 and 300 <= lag <= 5000})
        runs[seed] = run
    med = lambda v: statistics.median(v) if v else None  # noqa: E731
    vals = lambda k: [r[k] for r in runs.values() if r.get(k) is not None]  # noqa: E731
    s = counts[END]
    cycle_runs = sum(1 for r in runs.values() if r.get("cyclic"))
    return {
        "at_20000": counts[MID],
        "at_40000": s,
        "survival": {
            "carnivore_extinct_le_2": s["carnivore_extinct"] <= 2,
            "herbivore_extinct_le_2": s["herbivore_extinct"] <= 2,
            "coexist_ge_6": s["coexist"] >= 6,
        },
        "cycle_runs": cycle_runs,
        "cycle": s["coexist"] > 0 and cycle_runs * 2 >= s["coexist"],
        "carnivore_extinct_tick_median": med(vals("carnivore_extinct_tick")),
        "carnivore_peak_median": med(vals("carnivore_peak")),
        "carnivore_min_after_peak_median": med(vals("carnivore_min_after_peak")),
        "herbivore_min_after_carnivore_peak_median": med(vals("herbivore_min_after_carnivore_peak")),
        "kills_median": med(vals("kills")),
        "carnivore_digested_per_tick_median": med(vals("carnivore_digested_per_tick")),
        "carnivore_movement_per_tick_median": med(vals("carnivore_movement_per_tick")),
        "runs": runs,
    }


def control_matches(directory, ref_dir, arm, ref_arm):
    bad = []
    for seed in SEEDS:
        mine = json.loads((Path(directory) / f"{arm}_s{seed}.json").read_text())["rows"]
        ref = json.loads((Path(ref_dir) / f"{ref_arm}_s{seed}.json").read_text())["rows"]
        n = min(len(mine), len(ref))
        if [r[:4] for r in mine[:n]] != [r[:4] for r in ref[:n]]:
            bad.append(seed)
    return bad


def main():
    directory, f40c, f40 = sys.argv[1], sys.argv[2], sys.argv[3]
    result = {"control_mismatched_seeds": control_matches(directory, f40c, "m_0072", "f40_E")
              + control_matches(directory, f40, "m_0144", "f40_v525_rest")}
    result.update({arm: summarize(directory, arm) for arm in ARMS})
    ok = [a for a in CANDIDATES if all(result[a]["survival"].values()) and result[a]["cycle"]]
    result["candidates"] = ok
    result["selected"] = (
        min(ok, key=lambda a: (-result[a]["at_40000"]["coexist"], -result[a]["cycle_runs"], CANDIDATES.index(a)))
        if ok and not result["control_mismatched_seeds"] else None
    )
    print(json.dumps(result, indent=1, ensure_ascii=False, default=lambda x: None))


if __name__ == "__main__":
    main()
