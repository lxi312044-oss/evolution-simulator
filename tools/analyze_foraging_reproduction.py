"""Join intake decisions, nutrient digestion, reserve transactions and reproduction gates."""
import argparse
import json
import math
from collections import Counter, defaultdict
from itertools import zip_longest
from pathlib import Path

from analyze_reproduction_health import audit_run, load, read_rows
from run_confirmation import require
from run_metadata import sha256


def close(a, b):
    return math.isclose(a, b, rel_tol=0, abs_tol=1e-8)


def reasons(row):
    feeding = row["feeding"]
    if feeding["gained_e"] > 0:
        return []
    found = []
    if row["plant_protected"]:
        found.append("plant_protection")
    if row["satiety_blocked"]:
        found.append("satiety_pause")
    if row["action"] == "Flee":
        found.append("fleeing")
    target = row["target"]
    if row["may_feed"] and target is None:
        found.append("no_food_in_view" if row["visible_n"] == 0 else "visible_but_no_target")
    if target is not None and not target["in_range"]:
        found.append("approaching" if row["action"] == "Pursue" else "out_of_range")
    if feeding["attempted"]:
        if feeding["available_plant_n"] <= 0:
            found.append("target_n_exhausted")
        if feeding["available_plant_e"] <= 0:
            found.append("target_e_exhausted")
        if feeding["free_stomach_n"] == 0:
            found.append("no_stomach_space")
        if feeding["energy_capacity"] <= 0:
            found.append("reserve_full")
    return found or ["unknown"]


def summarize(path, config, case, old_log, previous):
    audit, health, births, states, quality = audit_run(path, Path(case["source"]), config, case)
    require(sha256(old_log) == previous["quality"]["log_sha256"], "Old log hash mismatch")
    require(all(a == b for a, b in zip_longest(
        health, (row for row in read_rows(old_log) if row["kind"] == "health"))),
        "Earlier health and reproduction trace changed")
    require(sorted(s["id"] for s in states) == case["ids"], "Cohort changed")
    gates = {(tuple(row["id"]), row["tick"]): row for row in health
             if row["phase"] in {"reproduction_gates", "capacity_blocked"}}
    energy = defaultdict(list)
    decisions = {}
    for row in read_rows(path):
        if row["kind"] in {"energy", "foraging"}:
            key = (tuple(row["id"]), row["tick"])
            if row["kind"] == "energy":
                energy[key].append(row)
            else:
                require(key not in decisions, "Duplicate decision")
                decisions[key] = row
    require(set(decisions) == set(energy), "Energy transaction coverage differs")
    require(len(decisions) == previous["summary"]["ticks"], "Missing observed individual ticks")
    h = config["herbivores"]
    cut = h["stomach_capacity_n"] * h["reproduction_satiety_threshold"]
    birth_ticks = defaultdict(list)
    for row in births:
        if row["tick"] > case["start"]:
            birth_ticks[tuple(row["parent"])].append(row["tick"])
    individuals = {tuple(ident): Counter() for ident in case["ids"]}
    combinations, timing, spending = Counter(), Counter(), Counter()
    contexts = defaultdict(Counter)
    runs = Counter()
    last_reasons = {}
    reset_events = []
    for (ident, tick), row in sorted(decisions.items()):
        totals = individuals[ident]
        totals["ticks"] += 1
        ledger = energy[(ident, tick)]
        before = row["reserve_e_before_tick"]
        for event in ledger:
            require(close(event["before_e"], before), "Discontinuous reserve ledger")
            require(close(event["after_e"] - event["before_e"], event["delta_e"]), "Energy delta mismatch")
            before = event["after_e"]
            spending[event["phase"]] += event["delta_e"]
            totals["E_" + event["phase"]] += event["delta_e"]
        require(ledger[-1]["phase"] in {"end_tick", "removal"}, "Missing reserve checkpoint")
        require(close(row["stomach_n_before_digest"] - row["digested_n"], row["stomach_n_before_feeding"]),
                "Digestion does not reconcile")
        intake = row["feeding"]
        gain = intake["gained_e"]
        require(close(gain, sum(e["delta_e"] for e in ledger if e["phase"] == "feeding")),
                "Feeding energy mismatch")
        if intake["attempted"]:
            require(close(intake["eaten_n"], intake["absorbed_n"] + intake["waste_n"]),
                    "Feeding nitrogen mismatch")
        totals["digested_n"] += row["digested_n"]
        totals["eaten_n"] += intake["eaten_n"]
        totals["absorbed_n"] += intake.get("absorbed_n", 0)
        totals["waste_n"] += intake.get("waste_n", 0)
        totals["feeding_e"] += gain
        totals["ate_n_without_e"] += intake["eaten_n"] > 0 and gain == 0
        cause = reasons(row)
        if cause:
            totals["zero_e_ticks"] += 1
            combinations["+".join(cause)] += 1
            totals["overlap"] += len(cause) > 1
            totals["unknown"] += cause == ["unknown"]
            for tag in cause:
                totals["zero_" + tag] += 1
        else:
            totals["positive_e_ticks"] += 1
        pause_low = row["satiety_blocked"] and row["reserve_e"] < row["energy_threshold"]
        totals["paused_below_e_threshold"] += pause_low
        runs[ident] = runs[ident] + 1 if pause_low else 0
        totals["longest_paused_below_e"] = max(totals["longest_paused_below_e"], runs[ident])
        totals["feeding_stop"] += row["feeding_active_before"] and not row["feeding_active"]
        totals["feeding_resume"] += not row["feeding_active_before"] and row["feeding_active"]
        if row["streak_before"] > 0 and row["streak_after"] == 0:
            crossed = row["stomach_n_before_digest"] >= cut > row["stomach_n_before_feeding"]
            timing["resets"] += 1
            timing["digest_crossed_threshold"] += crossed
            timing["already_below_before_digest"] += row["stomach_n_before_digest"] < cut
            timing["paused_on_reset"] += row["satiety_blocked"]
            timing["refed_on_reset"] += intake["eaten_n"] > 0
            reset_events.append(dict(id=list(ident), tick=tick, before=row["stomach_n_before_digest"],
                                     after_digest=row["stomach_n_before_feeding"],
                                     digested_n=row["digested_n"], streak_before=row["streak_before"],
                                     paused=row["satiety_blocked"], previous_tick_reasons=last_reasons.get(ident),
                                     zero_income_reasons=cause, eaten_n=intake["eaten_n"], feeding_e=gain))
        last_reasons[ident] = cause
        gate = gates.get((ident, tick))
        if gate is None:
            continue
        d = gate["details"]
        totals["ate_n_but_e_gate_unmet"] += intake["eaten_n"] > 0 and not d["energy_ok"]
        totals["positive_income_but_e_gate_unmet"] += gain > 0 and not d["energy_ok"]
        if not all(d[k] for k in ("age_ok", "hp_ok", "cooldown_ok", "body_n_ok", "reserve_n_ok")):
            continue
        e_ok = d["energy_ok"] and d["transfer_e_ok"]
        both = e_ok and d["satiety_ok"]
        groups = ["other_basic_ready"]
        if any(0 <= birth - tick < 60 for birth in birth_ticks[ident]):
            groups.append("birth_preceding_60")
        if not birth_ticks[ident]:
            groups.append("no_birth_in_window")
        for group in groups:
            c = contexts[group]
            c["ticks"] += 1
            c["capacity_blocked"] += gate["phase"] == "capacity_blocked"
            c["e_and_satiety_ready"] += both
            c["e_only_unmet"] += not e_ok and d["satiety_ok"]
            c["satiety_only_unmet"] += e_ok and not d["satiety_ok"]
            c["both_unmet"] += not e_ok and not d["satiety_ok"]
            c["paused_below_e"] += pause_low
            c["ate_n_but_e_unmet"] += intake["eaten_n"] > 0 and not e_ok
            c["feeding_e"] += gain
    return dict(individuals=[dict(id=list(i), births=birth_ticks[i], **c) for i, c in individuals.items()],
                zero_reason_combinations=dict(combinations), reset_summary=dict(timing),
                reset_events=reset_events, reserve_transactions=dict(spending),
                reproductive_contexts={k: dict(v) for k, v in contexts.items()}, quality=quality)


def main(directory, previous, cohort):
    plan = load(directory / "plan.json")
    old = load(cohort / "assessment.json")
    capacity = load(previous / "assessment.json")
    results = {}
    for name, case in plan["cases"].items():
        if not case["ids"]:
            continue
        config = Path(case["config"])
        require(sha256(config) == case["input_sha256"], "Input changed")
        reference = dict(old[name], quality=capacity[name]["quality"])
        results[name] = summarize(directory / f"{name}.jsonl.gz", load(config), case,
                                  previous / f"{name}.jsonl", reference)
        print(name, len(results[name]["individuals"]), flush=True)
    with (directory / "assessment.json").open("x", encoding="utf-8") as stream:
        json.dump(results, stream, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("previous", type=Path)
    parser.add_argument("cohort", type=Path)
    args = parser.parse_args()
    main(args.directory, args.previous, args.cohort)
