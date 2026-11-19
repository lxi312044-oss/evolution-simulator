"""Issue #11: does stopping herbivore speed evolution let carnivores persist? (40x40, 8 seeds)

Plan: https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6980802991

Usage: python tools/analyze_no_speed_evolution.py <result-dir> <issue11-f40-result-dir>

Judgement on s_fix25:
- "speed evolution is decisive": carnivores extinct in <=4/8 and 3-species coexistence at
  tick 40000 in >=4/8.
- "something else is the main cause": carnivores extinct in >=7/8 and the median carnivore
  extinction tick is less than 2000 ticks later than s_ctrl.
- otherwise "both".
"""
import json
import statistics
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from analyze_counterattack import END, KILLS, MID, status  # noqa: E402
from analyze_stage4_carnivore_cycles import herbivore_peaks, lagged_correlation  # noqa: E402

ARMS = ["s_ctrl", "s_mutoff", "s_fix25"]
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


def control_matches(directory, f40):
    bad = []
    for seed in SEEDS:
        mine = json.loads((Path(directory) / f"s_ctrl_s{seed}.json").read_text())["rows"]
        ref = json.loads((Path(f40) / f"f40_E_s{seed}.json").read_text())["rows"]
        n = min(len(mine), len(ref))
        if [r[:4] for r in mine[:n]] != [r[:4] for r in ref[:n]]:
            bad.append(seed)
    return bad


def main():
    directory, f40 = sys.argv[1], sys.argv[2]
    result = {"control_mismatched_seeds": control_matches(directory, f40)}
    result.update({arm: summarize(directory, arm) for arm in ARMS})
    fix, ctrl = result["s_fix25"], result["s_ctrl"]
    extinct = fix["at_40000"]["carnivore_extinct"]
    delay = (fix["carnivore_extinct_tick_median"] or END) - (ctrl["carnivore_extinct_tick_median"] or END)
    if extinct <= 4 and fix["at_40000"]["coexist"] >= 4:
        verdict = "speed_evolution_decisive"
    elif extinct >= 7 and delay < 2000:
        verdict = "other_cause_main"
    else:
        verdict = "both"
    result["verdict_s_fix25"] = verdict
    result["s_fix25_extinction_delay_vs_ctrl"] = delay
    print(json.dumps(result, indent=1, ensure_ascii=False, default=lambda x: None))


if __name__ == "__main__":
    main()
