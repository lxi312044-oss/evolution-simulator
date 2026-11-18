"""Issue #11: carnivores born from herbivore offspring (simple version), 40x40, 8 seeds.

Plan: https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6985033351

Usage: python tools/analyze_carnivore_emergence.py <result-dir> <f40c-result-dir>

P: carnivores present in >=75% of ticks 10000-40000 in >=6/8 seeds.
H: herbivores and plants alive at tick 40000 in >=6/8 seeds.
C: among runs with herbivores and plants alive at 40000, at least half meet the Issue 11
   cycle criterion. Candidates: v_p002, v_p01, v_p01_b120 meeting P, H and C.
"""
import json
import statistics
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from analyze_stage4_carnivore_cycles import herbivore_peaks, lagged_correlation  # noqa: E402

ARMS = ["v_ctrl", "v_p002", "v_p01", "v_p01_b120"]
CANDIDATES = ARMS[1:]
SEEDS = range(90001, 90009)
END = 40000
SPEED = 6


def establishments(rows, threshold=10):
    """Times carnivores rose from 0 (or the start) to >= threshold before falling to 0."""
    count, armed = 0, True
    for r in rows:
        if r[1] == 0:
            armed = True
        elif armed and r[1] >= threshold:
            count += 1
            armed = False
    return count


def summarize(directory, arm):
    runs = {}
    for seed in SEEDS:
        doc = json.loads((Path(directory) / f"{arm}_s{seed}.json").read_text())
        rows = doc["rows"]
        last = doc["last_tick"]
        if len(rows) != last // 100:
            raise SystemExit(f"incomplete run: {arm} {seed}")
        window = [r for r in rows if 10000 < r[0] <= END]
        # Ticks after a stop count as carnivore-free (the run ended with an extinction).
        present = sum(1 for r in window if r[1] > 0) / ((END - 10000) // 100)
        alive_end = last == END and rows[-1][2] > 0 and rows[-1][3] > 0
        at = {r[0]: r for r in rows}
        run = {
            "last_tick": last,
            "carnivore_presence_10000_40000": present,
            "herbivores_plants_alive_at_40000": alive_end,
            "herbivore_extinct_tick": next((r[0] for r in rows if r[2] == 0), None),
            "born_from_herbivores": doc.get("carnivores_born_from_herbivores"),
            "establishments_ge10": establishments(rows),
            "carnivore_peak": max(r[1] for r in rows),
            "herbivore_speed_every_5000": [at[t][6][SPEED] if t in at else None for t in range(5000, 40001, 5000)],
            "carnivores_every_2000": [at[t][1] if t in at else None for t in range(2000, 40001, 2000)],
            "herbivores_every_2000": [at[t][2] if t in at else None for t in range(2000, 40001, 2000)],
        }
        if alive_end:
            peaks = herbivore_peaks(rows)
            r, lag = lagged_correlation(rows)
            run.update({"herbivore_peaks": peaks, "lag_correlation": r, "lag_ticks": lag,
                        "cyclic": len(peaks) >= 2 and r is not None and r >= 0.3 and 300 <= lag <= 5000})
        runs[seed] = run
    alive = sum(1 for r in runs.values() if r["herbivores_plants_alive_at_40000"])
    cyclic = sum(1 for r in runs.values() if r.get("cyclic"))
    present = sum(1 for r in runs.values() if r["carnivore_presence_10000_40000"] >= 0.75)
    med = lambda v: statistics.median(v) if v else None  # noqa: E731
    out = {
        "P_seeds_carnivores_present_ge_75pct": present,
        "H_seeds_herbivores_plants_alive": alive,
        "C_cyclic_runs": cyclic,
        "P": present >= 6,
        "H": alive >= 6,
        "C": alive > 0 and cyclic * 2 >= alive,
        "carnivore_presence_median": med([r["carnivore_presence_10000_40000"] for r in runs.values()]),
        "born_from_herbivores_median": med([r["born_from_herbivores"] for r in runs.values()
                                            if r["born_from_herbivores"] is not None]),
        "establishments_median": med([r["establishments_ge10"] for r in runs.values()]),
        "carnivore_peak_median": med([r["carnivore_peak"] for r in runs.values()]),
        "runs": runs,
    }
    return out


def control_matches(directory, f40c):
    bad = []
    for seed in SEEDS:
        mine = json.loads((Path(directory) / f"v_ctrl_s{seed}.json").read_text())["rows"]
        ref = json.loads((Path(f40c) / f"f40_E_s{seed}.json").read_text())["rows"]
        n = min(len(mine), len(ref))
        if [r[:4] for r in mine[:n]] != [r[:4] for r in ref[:n]]:
            bad.append(seed)
    return bad


def main():
    directory, f40c = sys.argv[1], sys.argv[2]
    result = {"control_mismatched_seeds": control_matches(directory, f40c)}
    result.update({arm: summarize(directory, arm) for arm in ARMS})
    ok = [a for a in CANDIDATES if result[a]["P"] and result[a]["H"] and result[a]["C"]]
    result["candidates"] = ok
    result["selected"] = (
        min(ok, key=lambda a: (-result[a]["carnivore_presence_median"], -result[a]["C_cyclic_runs"]))
        if ok and not result["control_mismatched_seeds"] else None
    )
    print(json.dumps(result, indent=1, ensure_ascii=False, default=lambda x: None))


if __name__ == "__main__":
    main()
