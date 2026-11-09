"""Audit the three paired focal replays and trace the actual first birth difference."""
import argparse
import json
import math
from collections import Counter, defaultdict
from itertools import zip_longest
from pathlib import Path

from analyze_reproduction_health import load, read_rows
from run_confirmation import require
from run_longterm_resistance import Audit
from run_metadata import sha256

LINEAGE = {"birth", "removal", "snapshot"}
EARLY_GATES = ("age_ok", "cooldown_ok", "hp_ok", "energy_ok", "satiety_ok")
LATE_GATES = ("body_n_ok", "reserve_n_ok", "transfer_e_ok")


def comparable(rows, end):
    for row in rows:
        kind = row["kind"]
        if kind not in LINEAGE | {"probe"} or row["tick"] > end:
            continue
        if kind == "snapshot" and row["tick"] % 1000 != 0:
            continue
        if kind == "probe":
            row = {key: row[key] for key in ("kind", "tick", "focal", "plants", "toxic_plants")}
        yield row


def read_arm(path, case):
    source = Path(case["source"])
    config = Path(case["input"])
    require(sha256(source) == case["source_sha256"], "Source log changed")
    require(sha256(config) == case["input_sha256"], "Input changed")
    ident, end = case["id"], case["end"]
    # The old worker writes its end-of-tick probe before draining lineage events.
    current = list(comparable(read_rows(path), end))
    archived = list(comparable(read_rows(source), end))
    for is_probe in (True, False):
        require(all(x == y for x, y in zip_longest(
            (row for row in current if (row["kind"] == "probe") == is_probe),
            (row for row in archived if (row["kind"] == "probe") == is_probe))),
            "Archived birth/removal/state trajectory differs")
    audit = Audit(load(config), end)
    health, energy = defaultdict(list), defaultdict(list)
    probes, foraging = {}, {}
    births = []
    initial = checks = terminal = None
    for row in read_rows(path):
        kind = row["kind"]
        if kind == "focal_initial":
            initial = row["individual"]
        elif kind == "probe":
            probes[row["tick"]] = row
        elif kind == "health":
            require(row["id"] == ident, "Wrong health ID")
            health[row["tick"]].append(row)
        elif kind == "energy":
            require(row["id"] == ident, "Wrong reserve ID")
            energy[row["tick"]].append(row)
        elif kind == "foraging":
            require(row["id"] == ident, "Wrong foraging ID")
            foraging[row["tick"]] = row
        elif kind == "shadow_checks":
            checks = row["ticks"]
        else:
            audit.accept(row)
            if kind == "birth" and row["parent"] == ident:
                births.append(row["tick"])
            if kind == "removal" and row["id"] == ident:
                terminal = row
    audit.finish()
    require(checks == sorted(set(range(0, end+1, 1000)) | {end} | set(case["check_ticks"])),
            "Missing state/RNG checks")
    require(initial is not None and initial["id"] == ident and terminal is not None, "Missing focal endpoint")
    require(list(probes) == list(range(end+1)) and len(births) == terminal["births"], "Incomplete focal run")
    require(abs(initial["resistance"] - case["resistance"]) < 1e-7, "Incorrect resistance arm")
    return dict(initial=initial, health=health, energy=energy, probes=probes, foraging=foraging,
                births=births, terminal=terminal,
                quality=dict(checks=checks, n=audit.complete["max_n_error"],
                             e=audit.complete["max_e_error"], log_sha256=sha256(path)))


def crossing(arm):
    previous = arm["initial"]["hp"]
    limit = arm["initial"]["hp_threshold"]
    require(previous >= limit, "Focal started below threshold")
    losses = Counter()
    result = None
    sequence = 0
    for tick, rows in arm["health"].items():
        for row in rows:
            require(row["seq"] > sequence, "Health event order changed")
            sequence = row["seq"]
            if row.get("hp_before") is None or row["hp_before"] == row["hp_after"]:
                continue
            require(previous == row["hp_before"], "Unobserved HP transition")
            require(row["phase"] in {"toxin", "starvation", "predation"}, "Unknown HP writer")
            losses[row["phase"]] += previous - row["hp_after"]
            previous = row["hp_after"]
            if result is None and previous < limit:
                result = dict(event=row, cumulative_loss=dict(losses), events_at_tick=rows)
        probe = arm["probes"][tick]["focal"]
        if probe:
            require(probe["hp"] == previous, "HP checkpoint differs")
    require(previous == arm["terminal"]["hp"], "Terminal HP differs")
    return result


def gate_at(arm, tick):
    rows = arm["health"][tick]
    gate = next((r for r in rows if r["phase"] in ("reproduction_gates", "capacity_blocked")), None)
    food = next((r["details"] for r in rows if r["phase"] == "food_gate"), None)
    born = tick in arm["births"]
    if gate is None:
        reason = "removed_before_evaluation" if arm["terminal"]["tick"] == tick else "unobserved"
    elif gate["phase"] == "capacity_blocked":
        reason = "capacity"
    else:
        d = gate["details"]
        reason = next((flag for flag in EARLY_GATES if not d[flag]), None)
        if reason is None:
            require(food is not None, "Food check missing after early gates passed")
            reason = "food_gate" if not food["passed"] else next((flag for flag in LATE_GATES if not d[flag]), None)
        require((reason is None) == born, "First failure does not reconcile with birth")
    return dict(gate=gate, food=food, first_block=reason, birth=born,
                foraging=arm["foraging"].get(tick), post_tick=arm["probes"][tick]["focal"])


def budget(arm, start, end, gate):
    if gate is None:
        return None
    balance = arm["probes"][start]["focal"]["reserve_e"]
    total = Counter()
    for tick in range(start + 1, end + 1):
        for row in arm["energy"][tick]:
            if tick == end and row["health_seq"] >= gate["seq"]:
                break
            require(math.isclose(balance, row["before_e"], abs_tol=1e-8, rel_tol=0),
                    "Reserve transactions do not chain")
            balance = row["after_e"]
            total[row["phase"]] += row["delta_e"]
    require(math.isclose(balance, gate["details"]["reserve_e"], abs_tol=1e-8, rel_tol=0),
            "Reserve does not reconcile at reproduction gate")
    return dict(start_tick=start, start_e=arm["probes"][start]["focal"]["reserve_e"],
                end_tick=end, gate_e=balance, transactions=dict(total))


def main(directory, output):
    plan = load(directory / "plan.json")
    pairs = defaultdict(dict)
    for name, case in plan["cases"].items():
        pairs[case["seed"]][str(case["resistance"])] = read_arm(directory / f"{name}.jsonl", case)
        print(name, "trajectory and state checks passed", flush=True)
    report = {}
    for seed, arms in pairs.items():
        low, high = arms["0.2"], arms["0.8"]
        differences = sorted(set(low["births"]) ^ set(high["births"]))
        first = differences[0]
        case = plan["cases"][f"{seed}_0.9_0.2"]
        require(first == case["first_birth_difference"], "First birth difference changed")
        start = max((t for t in set(low["births"]) & set(high["births"]) if t < first), default=0)
        events = {}
        for tick in differences:
            if not any(a <= tick <= b for a, b in case["windows"]):
                continue
            events[tick] = {}
            for name, arm in arms.items():
                g = gate_at(arm, tick)
                g["reserve_budget"] = budget(arm, start, tick, g["gate"])
                g["reserve_budget_from_initial"] = budget(arm, 0, tick, g["gate"])
                events[tick][name] = g
        equal_dose = all(low["probes"][t]["focal"]["dose"] == high["probes"][t]["focal"]["dose"]
                         for t in range(first + 1))
        report[seed] = dict(first_birth_difference=first, birth_events=events,
                            identical_dose_history_through_first_difference=equal_dose,
                            arms={k: dict(initial=v["initial"], births=v["births"], terminal=v["terminal"],
                                          first_hp_crossing=crossing(v), quality=v["quality"])
                                  for k, v in arms.items()})
    with output.open("x", encoding="utf-8") as stream:
        json.dump(report, stream, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    main(args.directory, args.output or args.directory / "assessment.json")
