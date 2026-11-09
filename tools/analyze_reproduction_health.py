"""Audit health traces and separate recorded reproduction blocks from untested gates."""
import argparse
import json
from collections import Counter
from itertools import zip_longest
from pathlib import Path

from run_confirmation import require
from run_longterm_resistance import Audit, f32
from run_metadata import sha256


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def comparable(rows, end):
    for row in rows:
        kind = row["kind"]
        if kind in {"birth", "removal", "population", "snapshot"} and row["tick"] <= end:
            if kind != "snapshot" or row["tick"] % 1000 == 0:
                yield row


def read_rows(path):
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            yield json.loads(line)


def audit_run(path, source, config, case):
    end = case["end"]
    audit = Audit(config, end)
    require(sha256(source) == case["source_sha256"], "Source log changed")
    require(all(a == b for a, b in zip_longest(comparable(read_rows(path), end),
                                               comparable(read_rows(source), end))),
            "Archived population or lineage trajectory differs")
    health, births, start_status = [], [], []
    checked = None
    for row in read_rows(path):
        kind = row["kind"]
        if kind == "health":
            require(row["seq"] == len(health) + 1, "Health event order or count differs")
            health.append(row)
        elif kind == "cohort":
            require(row["tick"] == case["start"], "Unexpected cohort checkpoint")
            start_status = row["individuals"]
        elif kind == "shadow_checks":
            checked = row["count"]
        elif kind != "population":
            audit.accept(row)
            if kind == "birth":
                births.append(row)
    audit.finish()
    expected_checks = len(set(range(0, end + 1, 1000)) | {end} |
                          ({case["start"]} if case["start"] % 100 == 0 else set()))
    require(checked == (expected_checks if case["shadow"] else 0), "Missing state/RNG checks")
    quality = dict(log_sha256=sha256(path), shadow_checks=checked,
                   max_n_error=audit.complete["max_n_error"], max_e_error=audit.complete["max_e_error"])
    return audit, health, births, start_status, quality


def reproduction(events, births, ident, ticks, after_sequence=0):
    gates, capacity, removed, food = {}, set(), set(), {}
    for row in events:
        if row["id"] != ident or row["tick"] not in ticks or row["seq"] <= after_sequence:
            continue
        phase, tick = row["phase"], row["tick"]
        if phase == "reproduction_gates":
            require(tick not in gates, "Repeated gate evaluation")
            gates[tick] = row["details"]
        elif phase == "capacity_blocked":
            capacity.add(tick)
        elif phase == "pre_removal":
            removed.add(tick)
        elif phase == "food_gate":
            food[tick] = row["details"]["passed"]
    require(not (set(gates) & capacity or set(gates) & removed or capacity & removed),
            "Conflicting evaluation reasons")
    patterns = Counter()
    hp_only = hp_joint = 0
    for gates_at_tick in gates.values():
        failed = sorted(key for key, value in gates_at_tick.items() if key.endswith("_ok") and not value)
        patterns["+".join(failed) or "all_recorded_pass"] += 1
        hp_only += failed == ["hp_ok"]
        hp_joint += "hp_ok" in failed and len(failed) > 1
    direct = [row["tick"] for row in births if row["parent"] == ident and row["tick"] in ticks]
    require(all(tick in gates for tick in direct), "Birth without an evaluated parent")
    require(set(food) <= set(gates), "Food check without base gate evaluation")
    return dict(ticks=len(ticks), evaluated=len(gates), hp_only=hp_only, hp_joint=hp_joint,
                failures=dict(patterns), capacity_blocked=len(capacity), removed_before_gate=len(removed),
                unexplained_unassessed=len(ticks - set(gates) - capacity - removed),
                food_pass=sum(food.values()), food_fail=len(food) - sum(food.values()),
                food_not_tested=len(gates) - len(food), birth_ticks=direct)


def threshold(audit, health, births, ident, end, config):
    relevant = [row for row in health if row["id"] == ident]
    birth = next(row for row in relevant if row["phase"] == "birth")
    limit = f32(birth["adult_max_hp"] * f32(config["herbivores"]["reproduction_min_hp_fraction"]))
    changes = [row for row in relevant if row.get("hp_before") is not None and
               row["hp_before"] != row["hp_after"]]
    previous = birth["hp_after"]
    total = Counter()
    at_crossing = None
    crossing = None
    for row in changes:
        require(row["hp_before"] == previous, "Unobserved HP transition")
        require(row["phase"] in {"toxin", "starvation", "predation"}, "Unknown HP source")
        previous = row["hp_after"]
        total[row["phase"]] += row["hp_before"] - previous
        if crossing is None and row["hp_before"] >= limit > previous:
            crossing = row
            at_crossing = dict(total)
    require(crossing is not None, "No threshold crossing observed")
    require(previous == audit.survivors[tuple(ident)]["hp"], "HP history does not reconcile at end")
    prior_births = [row["tick"] for row in births if row["parent"] == ident]
    require(14063 in prior_births, "Archived focal birth not reproduced")
    ticks = set(range(crossing["tick"], end + 1))
    # If the crossing followed that tick's evaluation, it cannot block that evaluation.
    if any(row["tick"] == crossing["tick"] and row["seq"] < crossing["seq"] and
           row["phase"] in {"reproduction_gates", "capacity_blocked"} for row in relevant):
        ticks.remove(crossing["tick"])
    gates = reproduction(health, births, ident, ticks, crossing["seq"])
    require(gates["unexplained_unassessed"] == 0, "Unclassified reproduction evaluation")
    return dict(birth=birth, hp_threshold=limit, crossing=crossing, loss_through_crossing=at_crossing,
                loss_through_end=dict(total), same_tick_events=[r for r in relevant if r["tick"] == crossing["tick"]],
                focal_birth_ticks=prior_births, post_crossing=gates)


def main(directory, source, config_path):
    case = load(directory / "case.json")
    config = load(config_path)
    require(sha256(config_path) == case["input_sha256"], "Input changed")
    audit, health, births, _, quality = audit_run(directory / "observed.jsonl", source, config, case)
    result = threshold(audit, health, births, case["ids"][0], case["end"], config)
    with (directory / "assessment.json").open("x", encoding="utf-8") as stream:
        json.dump(dict(result=result, quality=quality), stream, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("source", type=Path)
    parser.add_argument("config", type=Path)
    args = parser.parse_args()
    main(args.directory, args.source, args.config)
