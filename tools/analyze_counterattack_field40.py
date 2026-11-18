"""Issue #54: counterattack and carnivore retreat on the 40x40 field (f40r_* arms, 8 seeds).

Plan: https://github.com/lxi312044-oss/evolution-simulator/issues/54#issuecomment-6978567155

Usage: python tools/analyze_counterattack_field40.py <result-dir> <issue11-f40-result-dir>

M1: learning in >=6/8 seeds. S: carnivore extinct <=2/8, herbivore extinct <=2/8, coexist >=6/8 (tick 40000), plus the
Issue 11 cycle criterion for coexisting runs. M3: strong fraction up at tick 8000 in >=6/8.

Same M1 / M2 / S as tools/analyze_counterattack.py. M2 is "undecidable" when carnivores
went extinct before tick 5000 in at least half of the seeds. Mimicry signs (descriptive):
among low-aggression herbivores, Spearman correlation of color distance to the mean color
of strong herbivores (aggression x hp / 60 >= 0.7) with lifetime births.
"""
import json
import math
import statistics
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from analyze_counterattack import (  # noqa: E402
    AGGR, BIRTHS, COLOR_SKIPS, END, KILLS, MID, quartile_ratio, spearman, status,
)
from analyze_stage4_carnivore_cycles import herbivore_peaks, lagged_correlation  # noqa: E402

SEEDS = range(90001, 90009)

ARMS = ["f40r_ctrl", "f40r_c2_th_ret", "f40r_c2_th_dec_ret"]
RETREAT_ARMS = ARMS[1:]
LEARNING_ARMS = ARMS[1:]
STRONG = 0.7


def strength(d):
    return d[AGGR] * d[0] / 60


def mimicry_signs(individuals):
    strong = [d for d in individuals if strength(d) >= STRONG]
    if len(strong) < 4 or len(individuals) < 16:
        return None
    model = [statistics.mean(d[i] for d in strong) for i in (4, 5, 6)]
    ranked = sorted(individuals, key=lambda d: d[AGGR])
    low = ranked[: len(ranked) // 2]
    return spearman([math.dist(model, d[4:7]) for d in low], [d[BIRTHS] for d in low])


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
        strong_series = [r[6][9] for r in rows if r[6][9] is not None]
        runs[seed] = {
            "last_tick": last,
            "carnivore_extinct_tick": next((r[0] for r in rows if r[1] == 0), None),
            "herbivore_extinct_tick": next((r[0] for r in rows if r[2] == 0), None),
            "carnivore_peak": rows[peak_i][1],
            "counterattacks": sum(r[6][0] for r in rows),
            "learning_events": sum(r[6][2] for r in rows),
            "color_skips": sum(r[4][COLOR_SKIPS] for r in rows),
            "kills": sum(r[4][KILLS] for r in rows),
            "strong_fraction_first_last": [strong_series[0], strong_series[-1]] if strong_series else None,
            "strong_fraction_start_8000": [rows[0][6][9], next((r[6][9] for r in rows if r[0] == 8000), None)],
            "aggression_births_ratio": quartile_ratio(individuals, AGGR),
            "mimicry_spearman": mimicry_signs(individuals),
        }
        cext = runs[seed]["carnivore_extinct_tick"]
        while_alive = [d for d in individuals if cext is None or d[-1] <= cext]
        runs[seed]["aggression_births_ratio_while_carnivores_alive"] = quartile_ratio(while_alive, AGGR)
        if status(rows, END)["coexist"]:
            peaks = herbivore_peaks(rows)
            r, lag = lagged_correlation(rows)
            runs[seed].update({"herbivore_peaks": peaks, "lag_correlation": r, "lag_ticks": lag,
                               "cyclic": len(peaks) >= 2 and r is not None and r >= 0.3 and 300 <= lag <= 5000})
    med = lambda v: statistics.median(v) if v else None  # noqa: E731
    vals = lambda k: [r[k] for r in runs.values() if r[k] is not None]  # noqa: E731
    s = counts[END]
    early = sum(1 for r in runs.values() if r["carnivore_extinct_tick"] is not None
                and r["carnivore_extinct_tick"] < 5000)
    m2 = med(vals("aggression_births_ratio"))
    out = {
        "at_20000": counts[MID],
        "at_40000": s,
        "M1_seeds_with_learning": sum(1 for r in runs.values() if r["learning_events"] > 0),
        "M2_aggression_births_ratio_median": m2,
        "seeds_carnivores_extinct_before_5000": early,
        "survival": {
            "carnivore_extinct_le_2": s["carnivore_extinct"] <= 2,
            "herbivore_extinct_le_2": s["herbivore_extinct"] <= 2,
            "coexist_ge_6": s["coexist"] >= 6,
        },
        "aggression_births_ratio_while_carnivores_alive_median": med(
            vals("aggression_births_ratio_while_carnivores_alive")),
        "cycle_runs": sum(1 for r in runs.values() if r.get("cyclic")),
        "carnivore_extinct_tick_median": med(vals("carnivore_extinct_tick")),
        "carnivore_peak_median": med(vals("carnivore_peak")),
        "counterattacks_median": med(vals("counterattacks")),
        "color_skips_median": med(vals("color_skips")),
        "kills_median": med(vals("kills")),
        "strong_fraction_first_median": med([r["strong_fraction_first_last"][0] for r in runs.values()
                                             if r["strong_fraction_first_last"]]),
        "strong_fraction_last_median": med([r["strong_fraction_first_last"][1] for r in runs.values()
                                            if r["strong_fraction_first_last"]]),
        "mimicry_spearman_median": med(vals("mimicry_spearman")),
        "runs": runs,
    }
    pairs = [r["strong_fraction_start_8000"] for r in runs.values()]
    out["M3_seeds_strong_fraction_up_at_8000"] = sum(1 for a, b in pairs if b is not None and b > a)
    out["strong_fraction_8000_median"] = med([b for _, b in pairs if b is not None])
    out["M3"] = out["M3_seeds_strong_fraction_up_at_8000"] >= 6
    out["M1"] = arm in LEARNING_ARMS and out["M1_seeds_with_learning"] >= 6
    out["M2"] = "undecidable" if early * 2 >= len(SEEDS) else (m2 is not None and m2 >= 1.0)
    out["cycle"] = s["coexist"] > 0 and out["cycle_runs"] * 2 >= s["coexist"]
    out["S"] = all(out["survival"].values())
    return out


def control_matches(directory, f40):
    bad = []
    for seed in SEEDS:
        mine = json.loads((Path(directory) / f"f40r_ctrl_s{seed}.json").read_text())["rows"]
        ref = json.loads((Path(f40) / f"f40_E_s{seed}.json").read_text())["rows"]
        n = min(len(mine), len(ref))
        if [r[:4] for r in mine[:n]] != [r[:4] for r in ref[:n]]:
            bad.append(seed)
    return bad


def main():
    directory, round3 = sys.argv[1], sys.argv[2]
    result = {"control_mismatched_seeds": control_matches(directory, round3)}
    result.update({arm: summarize(directory, arm) for arm in ARMS})
    result["candidates"] = [a for a in RETREAT_ARMS if result[a]["S"] and result[a]["cycle"]]
    print(json.dumps(result, indent=1, ensure_ascii=False, default=lambda x: None))


if __name__ == "__main__":
    main()
