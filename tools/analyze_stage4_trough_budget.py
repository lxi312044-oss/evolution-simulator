#!/usr/bin/env python3
"""Issue #11: energy budget of carnivores by phase (measurement only).

Plan: https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6973191538

Usage: analyze_stage4_trough_budget.py <trough-results-dir> <f40-results-dir>

Phases per run: increase (start to carnivore peak), decline (after the peak until
carnivores <= 20% of the peak), trough (after that until extinction).
Intake from hunting per carnivore-tick = encounter (visible ticks / tick)
x kill rate (kills / visible tick) x energy per kill.
"""
import json
import math
import statistics
import sys
from pathlib import Path

PHASES = ["increase", "decline", "trough"]


def load(path):
    d = json.loads(Path(path).read_text())
    f = d["row_fields"]
    names = f[4] + f[5] + f[6]
    rows = []
    for r in d["rows"]:
        row = {"tick": r[0], "carnivores": r[1], "herbivores": r[2], "plants": r[3]}
        row.update(zip(names, r[4] + r[5] + r[6]))
        rows.append(row)
    return d, rows


def phase_of(rows):
    peak_i = max(range(len(rows)), key=lambda i: rows[i]["carnivores"])
    peak = rows[peak_i]["carnivores"]
    out = []
    in_trough = False
    for i, r in enumerate(rows):
        if i <= peak_i:
            out.append("increase")
        else:
            if r["carnivores"] <= 0.2 * peak:
                in_trough = True
            out.append("trough" if in_trough else "decline")
    return out, peak, rows[peak_i]["tick"]


def summarize(rows):
    s = lambda k: sum(r[k] for r in rows)
    ticks = s("phase17_rest_ticks") + s("phase17_active_search_ticks") + s("phase17_chase_ticks")
    if ticks == 0:
        return None
    visible = s("p41_visible_samples")
    kills = s("prey_hp_zero_after_attack")
    hunt_e = s("hunt_energy")
    w = sum(r["carnivores"] for r in rows) or 1
    snap = lambda k: sum(r[k] * r["carnivores"] for r in rows) / w
    return {
        "carnivore_ticks": ticks,
        "mean_herbivores": statistics.mean(r["herbivores"] for r in rows),
        "intake_digested": s("stomach_energy_digested") / ticks,
        "intake_hunt": hunt_e / ticks,
        "spend_basal": s("energy_basal_spent") / ticks,
        "spend_movement": s("energy_movement_spent") / ticks,
        "spend_reproduction": s("energy_reproduction_spent") / ticks,
        "rest_fraction": s("phase17_rest_ticks") / ticks,
        "search_fraction": s("phase17_active_search_ticks") / ticks,
        "chase_fraction": s("phase17_chase_ticks") / ticks,
        "encounter": visible / ticks,
        "kill_rate": kills / visible if visible else math.nan,
        "energy_per_kill": hunt_e / kills if kills else math.nan,
        "bites_per_kill": s("hunt_contacts") / kills if kills else math.nan,
        "energy_per_bite": hunt_e / s("hunt_contacts") if s("hunt_contacts") else math.nan,
        "bite_capacity_blocked_fraction": s("bite_stomach_capacity_blocked") / s("hunt_contacts") if s("hunt_contacts") else math.nan,
        "reserve_energy": snap("mean_reserve_energy"),
        "stomach_energy": snap("mean_stomach_energy"),
        "stomach_nutrients": snap("mean_stomach_nutrients"),
        "feeding_active_fraction": snap("feeding_active_fraction"),
        "nearest_herbivore_distance": snap("mean_nearest_herbivore_distance"),
        "herbivores_within_vision": snap("mean_herbivores_within_vision"),
        "energy_deaths": s("carnivore_energy_deaths"),
        "age_deaths": s("carnivore_age_deaths"),
        "repro_energy_blocked_share": s("reproduction_energy_blocked") / ticks,
        "births_spend_check": s("energy_reproduction_spent"),
    }


def main():
    tdir, fdir = Path(sys.argv[1]), Path(sys.argv[2])
    runs = {}
    mismatches = []
    for p in sorted(tdir.glob("trough_s*.json")):
        d, rows = load(p)
        seed = d["seed"]
        ref = json.loads((fdir / f"f40_E_s{seed}.json").read_text())["rows"]
        refmap = {r[0]: r[1:4] for r in ref}
        for r in rows:
            if refmap.get(r["tick"]) != [r["carnivores"], r["herbivores"], r["plants"]]:
                mismatches.append((seed, r["tick"]))
                break
        phases, peak, peak_tick = phase_of(rows)
        res = {"peak": peak, "peak_tick": peak_tick, "last_tick": d["last_tick"]}
        for ph in PHASES:
            res[ph] = summarize([r for r, q in zip(rows, phases) if q == ph])
        late = [r for r in rows if 0 < r["carnivores"] <= 5 and r["tick"] > peak_tick]
        res["survivors_le5"] = summarize(late)
        runs[seed] = res
    keys = [k for k in runs[next(iter(runs))]["increase"]]
    med = {}
    for ph in PHASES + ["survivors_le5"]:
        med[ph] = {}
        for k in keys:
            v = [runs[s][ph][k] for s in runs if runs[s][ph] and not math.isnan(runs[s][ph][k])]
            med[ph][k] = statistics.median(v) if v else None
    ratio = {}
    for k in ["encounter", "kill_rate", "energy_per_kill"]:
        v = [runs[s]["trough"][k] / runs[s]["increase"][k] for s in runs
             if runs[s]["trough"] and runs[s]["increase"][k] and not math.isnan(runs[s]["trough"][k])]
        ratio[k] = statistics.median(v) if v else None
    main_factor = min((k for k in ratio if ratio[k] is not None), key=lambda k: ratio[k])
    out = {"consistency_mismatches": mismatches, "median_by_phase": med,
           "trough_over_increase_median": ratio, "main_factor": main_factor, "runs": runs}
    print(json.dumps(out, indent=1, default=lambda x: None))


if __name__ == "__main__":
    main()
