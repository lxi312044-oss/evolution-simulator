"""Report each high-resistance trough individual and each seed/arm."""
import argparse
import json
from collections import Counter
from pathlib import Path

from analyze_reproduction_health import audit_run, load, reproduction
from run_confirmation import require
from run_metadata import sha256


def main(directory):
    plan = load(directory / "plan.json")
    arms = {}
    for name, case in plan["cases"].items():
        start, end = case["start"], case["end"]
        individuals = []
        quality = None
        if case["ids"]:
            config_path = Path(case["config"])
            require(sha256(config_path) == case["input_sha256"], "Configuration changed")
            audit, events, births, states, quality = audit_run(
                directory / f"{name}.jsonl", Path(case["source"]), load(config_path), case)
            require(sorted(s["id"] for s in states) == case["ids"], "Wrong trough cohort")
            for state in states:
                ident = state["id"]
                require(state["hp"] > 0, "Trough individual lacks positive HP")
                end_of_life = audit.life[tuple(ident)]["end"]
                last = end if end_of_life is None else end_of_life
                ticks = set(range(start + 1, last + 1))
                row = reproduction(events, births, ident, ticks)
                require(row["unexplained_unassessed"] == 0, "Unclassified evaluation gap")
                threshold = state["hp_threshold"]
                hp = [state["hp"]] + [e["hp_after"] for e in events if e["id"] == ident
                                       and start < e["tick"] <= end and "hp_after" in e]
                individuals.append(dict(**state, hp_below_at_trough=state["hp"] < threshold,
                                        hp_below_ever=min(hp) < threshold, minimum_hp=min(hp),
                                        removal_tick=end_of_life, ticks_after_removal=end-last, **row))
        summary = Counter()
        patterns = Counter()
        for row in individuals:
            for key in ("hp_below_at_trough", "hp_below_ever", "ticks", "evaluated", "hp_only", "hp_joint",
                        "capacity_blocked", "removed_before_gate", "unexplained_unassessed",
                        "ticks_after_removal", "food_pass", "food_fail", "food_not_tested"):
                summary[key] += row[key]
            summary["births"] += len(row["birth_ticks"])
            summary["breeders"] += bool(row["birth_ticks"])
            patterns.update(row["failures"])
        arms[name] = dict(arm=case["arm"], start=start, end=end, cohort=len(individuals),
                          below_at_trough_fraction=summary["hp_below_at_trough"] / len(individuals) if individuals else None,
                          below_ever_fraction=summary["hp_below_ever"] / len(individuals) if individuals else None,
                          summary=dict(summary), failures=dict(patterns),
                          individuals=individuals, quality=quality)
        print(name, len(individuals), dict(summary), flush=True)
    require([sum(a["cohort"] for a in arms.values() if a["arm"] == arm) for arm in ("on", "off")] ==
            [26, 10], "Incomplete cohorts")
    require([sum(a["summary"].get("births", 0) for a in arms.values() if a["arm"] == arm)
             for arm in ("on", "off")] == [9, 8], "Archived cohort birth totals differ")
    with (directory / "assessment.json").open("x", encoding="utf-8") as stream:
        json.dump(arms, stream, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    main(args.directory)
