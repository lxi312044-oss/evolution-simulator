"""Reconstruct all observed feeding pauses and subsequent recovery milestones."""
import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path

from analyze_foraging_reproduction import reasons
from analyze_reproduction_health import load, read_rows
from run_confirmation import require
from run_metadata import sha256


def first(rows, predicate):
    return next((row["tick"] for row in rows if predicate(row)), None)


def predicted_restart(stomach, config):
    capacity = config["stomach_capacity_n"]
    digest = config["digestion_n_per_tick"]
    require(digest > 0, "Restart estimate needs positive digestion")
    ticks = 0
    while stomach / capacity > config["feeding_resume_fraction"]:
        stomach -= min(stomach, digest)
        ticks += 1
    return ticks


def episodes(rows, config, end):
    stops = [i for i, r in enumerate(rows)
             if (i == 0 and not r["feeding_active_before"])
             or (r["feeding_active_before"] and not r["feeding_active"])]
    resets = [r["tick"] for r in rows if r["streak_before"] > 0 and r["streak_after"] == 0]
    result = []
    for i in stops:
        start = rows[i]["tick"]
        left = i == 0 and not rows[i]["feeding_active_before"]
        resume = first(rows[i:], lambda r: r["feeding_active"])
        pause = [r for r in rows[i:] if resume is None or r["tick"] < resume]
        reset = next((t for t in resets if start <= t and (resume is None or t <= resume)), None)
        later_reset = next((t for t in resets if t > (reset or start)
                            and (resume is None or t > resume)), None)
        cutoff = later_reset - 1 if later_reset else rows[-1]["tick"]
        end_reason = ("next_reset" if later_reset else
                      "removed" if "removal" in rows[-1]["energy"] else
                      "window_end" if rows[-1]["tick"] == end else "incomplete_trace")
        after_resume = [r for r in rows[i:] if resume is not None and resume <= r["tick"] <= cutoff]
        recovery_anchor = reset if reset is not None else resume
        recovery = [r for r in rows[i:] if recovery_anchor is not None
                    and recovery_anchor <= r["tick"] <= cutoff]
        arrival = first(after_resume, lambda r: r["feeding"]["attempted"])
        intake = first(after_resume, lambda r: r["feeding"]["eaten_n"] > 0)
        income = first(after_resume, lambda r: r["feeding"]["gained_e"] > 0)
        ready_e = first(recovery, lambda r: r["energy_ready"] is True)
        n_ready = first(recovery, lambda r: r["stomach_n_before_feeding"] >=
                        config["stomach_capacity_n"] * config["reproduction_satiety_threshold"])
        ready_s = first(recovery, lambda r: r["streak_after"] >= config["reproduction_well_fed_ticks"])
        both = first(recovery, lambda r: r["energy_ready"] is True and
                     r["streak_after"] >= config["reproduction_well_fed_ticks"])
        reset_row = next((r for r in pause if r["tick"] == reset), None)
        # If restart happens on the reset tick, use that tick's post-digestion value.
        if reset is not None and reset_row is None:
            reset_row = next(r for r in rows if r["tick"] == reset)
        search = [r for r in after_resume if arrival is None or r["tick"] < arrival]
        expenses = Counter()
        for r in pause:
            for phase, value in r["energy"].items():
                if value < 0:
                    expenses[phase] += -value
        result.append(dict(
            observed_start=start, stop_start=None if left else start, left_censored=left,
            reset_tick=reset, reset_status=("observed" if reset is not None else
                                           "possibly_before_window" if left else "unobserved"),
            resume_tick=resume, first_in_range_attempt_tick=arrival, first_intake_n_tick=intake,
            first_positive_e_tick=income, first_e_ready_from_pause=first(
                [r for r in rows[i:] if r["tick"] <= cutoff], lambda r: r["energy_ready"] is True),
            first_e_ready_after_reset_or_resume=ready_e, stomach_n_ready_tick=n_ready,
            satiety_60_tick=ready_s, both_ready_tick=both,
            e_ready_at_observed_start=rows[i]["energy_ready"],
            reset_to_resume=(resume-reset if resume is not None and reset is not None else None),
            digest_only_prediction=(predicted_restart(reset_row["stomach_n_before_feeding"], config)
                                    if reset_row is not None else None),
            resume_to_arrival=(arrival-resume if arrival is not None else None),
            resume_to_intake=(intake-resume if intake is not None else None),
            resume_to_satiety_60=(ready_s-resume if ready_s is not None and resume is not None else None),
            pause_ticks=len(pause), pause_e_expenses=dict(expenses),
            search_reason_combinations=dict(Counter("+".join(reasons(r)) for r in search)),
            recovery_e_income=sum(r["feeding"]["gained_e"] for r in recovery),
            recovery_n_intake=sum(r["feeding"]["eaten_n"] for r in recovery),
            end_tick=cutoff, end_reason=end_reason,
        ))
    return result


def summarize(rows, config):
    totals, transactions = Counter(), Counter()
    cut = config["stomach_capacity_n"] * config["reproduction_satiety_threshold"]
    for r in rows:
        totals["individual_ticks"] += 1
        totals["e_gate_observed"] += r["energy_ready"] is not None
        intake = r["feeding"]
        totals["eaten_n"] += intake["eaten_n"]
        totals["digested_n"] += r["digested_n"]
        totals["feeding_e"] += intake["gained_e"]
        totals["n_intake_without_e"] += intake["eaten_n"] > 0 and intake["gained_e"] == 0
        totals["n_threshold_met_e_unmet"] += (
            r["stomach_n_before_feeding"] >= cut and r["energy_ready"] is False)
        totals["satiety_60_met_e_unmet"] += (
            r["streak_after"] >= config["reproduction_well_fed_ticks"] and r["energy_ready"] is False)
        totals["positive_income_e_unmet"] += intake["gained_e"] > 0 and r["energy_ready"] is False
        if not r["feeding_active"]:
            totals["paused_ticks"] += 1
            totals["paused_e_unmet"] += r["energy_ready"] is False
            totals["paused_e_spent"] += sum(-v for v in r["energy"].values() if v < 0)
        transactions.update(r["energy"])
    return dict(totals=dict(totals), reserve_transactions=dict(transactions))


def main(directory, output):
    plan = load(directory / "plan.json")
    previous = load(directory / "assessment.json")
    runs = {}
    for name, case in plan["cases"].items():
        if not case["ids"]:
            runs[name] = dict(arm=case["arm"], individuals=[], totals={}, reserve_transactions={})
            continue
        path = directory / f"{name}.jsonl.gz"
        config_path = Path(case["config"])
        require(sha256(path) == previous[name]["quality"]["log_sha256"], "Observation log changed")
        require(sha256(config_path) == case["input_sha256"], "Configuration changed")
        config = load(config_path)["herbivores"]
        decisions, gates, energy = defaultdict(list), {}, defaultdict(Counter)
        for row in read_rows(path):
            if not case["start"] < row.get("tick", 0) <= case["end"]:
                continue
            if row["kind"] not in {"foraging", "health", "energy"}:
                continue
            ident, tick = tuple(row["id"]), row["tick"]
            if row["kind"] == "foraging":
                decisions[ident].append(row)
            elif row["kind"] == "energy":
                energy[(ident, tick)][row["phase"]] += row["delta_e"]
            elif row["phase"] in {"reproduction_gates", "capacity_blocked"}:
                d = row["details"]
                gates[(ident, tick)] = d["energy_ok"] and d["transfer_e_ok"]
        individuals = []
        totals, transactions = Counter(), Counter()
        for ident in map(tuple, case["ids"]):
            rows = sorted(decisions[ident], key=lambda r: r["tick"])
            require(rows and [r["tick"] for r in rows] ==
                    list(range(case["start"] + 1, rows[-1]["tick"] + 1)), "Incomplete focal chronology")
            for row in rows:
                key = (ident, row["tick"])
                row["energy_ready"] = gates.get(key)
                require(key in energy, "Missing energy ledger")
                row["energy"] = energy[key]
            result = summarize(rows, config)
            totals.update(result["totals"])
            transactions.update(result["reserve_transactions"])
            pauses = episodes(rows, config, case["end"])
            linked_resets = {episode["reset_tick"] for episode in pauses}
            unlinked_resets = [r["tick"] for r in rows
                               if r["streak_before"] > 0 and r["streak_after"] == 0
                               and r["tick"] not in linked_resets]
            individuals.append(dict(id=list(ident), **result, episodes=pauses,
                                    resets_outside_pause=unlinked_resets))
        runs[name] = dict(arm=case["arm"], individuals=individuals, totals=dict(totals),
                          reserve_transactions=dict(transactions))
    with output.open("x", encoding="utf-8") as stream:
        json.dump(runs, stream, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    main(args.directory, args.output)
