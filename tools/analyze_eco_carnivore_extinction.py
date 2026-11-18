"""Analyze the Issue #11 carnivore extinction diagnosis.

Criteria are fixed in the plan before the run:
https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6967911307

Usage: python tools/analyze_eco_carnivore_extinction.py <result-dir>
"""
import json
import sys
from pathlib import Path

ARMS = ["base", "carn_learn_off", "herb_decay", "herb_learn_off", "animal_toxin_off"]
SEEDS = range(90001, 90017)


def load(directory):
    runs = {}
    for arm in ARMS:
        for seed in SEEDS:
            path = Path(directory) / f"{arm}_s{seed}.json"
            if not path.exists():
                raise SystemExit(f"missing run: {path}")
            record = json.loads(path.read_text())
            if record["ticks"] != 20000 or len(record["rows"]) != 200:
                raise SystemExit(f"incomplete run: {path}")
            runs[arm, seed] = record
    return runs


def summarize(runs, arm):
    fields = runs[arm, SEEDS[0]]["row_fields"][4]
    total = dict.fromkeys(fields, 0)
    carnivore_ticks = 0
    extinct = {}
    herbivores_at_extinction = {}
    for seed in SEEDS:
        rows = runs[arm, seed]["rows"]
        extinct[seed] = next((r[0] for r in rows if r[1] == 0), None)
        herbivores_at_extinction[seed] = next((r[2] for r in rows if r[1] == 0), None)
        for _tick, carnivores, _herbivores, _plants, delta in rows:
            carnivore_ticks += carnivores * 100
            for key, value in zip(fields, delta):
                total[key] += value
    deaths = sum(total[k] for k in ["carnivore_energy_deaths", "carnivore_age_deaths", "carnivore_toxin_deaths"])
    per_k = lambda key: 1000 * total[key] / carnivore_ticks if carnivore_ticks else float("nan")
    share = lambda key: total[key] / deaths if deaths else float("nan")
    low_energy = total["prey_visible_while_low_energy"] + total["predator_low_energy_no_visible_prey"]
    gates = [k for k in fields if k.endswith("_blocked")]
    gate_total = sum(total[k] for k in gates)
    samples = total["predator_death_nearest_prey_samples"]
    return {
        "extinct": extinct,
        "extinct_count": sum(t is not None for t in extinct.values()),
        "herbivores_at_carnivore_extinction": herbivores_at_extinction,
        "carnivore_ticks": carnivore_ticks,
        "deaths": deaths,
        "death_share_energy": share("carnivore_energy_deaths"),
        "death_share_age": share("carnivore_age_deaths"),
        "death_share_toxin": share("carnivore_toxin_deaths"),
        "births_per_1000_carnivore_ticks": per_k("carnivore_births"),
        "hunt_attempts_per_1000": per_k("hunt_attempts"),
        "kills_per_1000": per_k("kills_with_meal"),
        "color_skips_per_1000": per_k("color_skips"),
        "low_energy_prey_visible_share": total["prey_visible_while_low_energy"] / low_energy if low_energy else None,
        "reproduction_block_shares": {k: total[k] / gate_total for k in gates} if gate_total else {},
        "mean_distance_to_nearest_prey_at_death": total["predator_death_distance_to_nearest_prey_total_x100"] / 100 / samples if samples else None,
        "totals": total,
    }


def rescue(result, arm):
    base = result["base"]["extinct"]
    other = result[arm]["extinct"]
    rescued = sum(base[s] is not None and other[s] is None for s in SEEDS)
    lost = sum(base[s] is None and other[s] is not None for s in SEEDS)
    met = result[arm]["extinct_count"] <= result["base"]["extinct_count"] - 4 and rescued > lost
    return {"rescued": rescued, "lost": lost, "criterion_met": met}


def main():
    runs = load(sys.argv[1])
    result = {arm: summarize(runs, arm) for arm in ARMS}
    for arm in ARMS[1:]:
        result[arm]["vs_base"] = rescue(result, arm)
    b = result["base"]
    result["hypotheses"] = {
        "H1_starvation_main_cause": b["death_share_energy"] >= 0.5,
        "H2_carnivore_learning": result["carn_learn_off"]["vs_base"]["criterion_met"],
        "H3_toxin": b["death_share_toxin"] >= 0.5 or result["animal_toxin_off"]["vs_base"]["criterion_met"],
        "H4_prey_dynamics": result["herb_learn_off"]["vs_base"]["criterion_met"]
        or result["herb_decay"]["vs_base"]["criterion_met"],
    }
    print(json.dumps(result, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
