"""Audit recovery upkeep intervention traces against the unchanged trajectory."""
import argparse
import json
from collections import Counter
from itertools import zip_longest
from pathlib import Path

from run_confirmation import require
from run_longterm_resistance import Audit
from run_metadata import sha256

EVENTS = {"birth", "removal", "snapshot"}


def baseline_events(path, end):
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line)
            if row["kind"] in EVENTS and row["tick"] <= end:
                yield row


def summarize(path, config, case):
    start, end, ident = case["start"], case["end"], case["id"]
    audit = Audit(config, end)
    probes, trajectory, direct = [], [], []
    intervention = terminal = None
    gates, blocks, energy = Counter(), Counter(), Counter()
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line)
            kind = row["kind"]
            if kind == "intervention":
                require(intervention is None and row["id"] == ident and row["tick"] == start,
                        "Wrong intervention")
                intervention = row
            elif kind == "probe":
                probes.append(row)
                for event in row["energy"]:
                    require(event["id"] == ident, "Wrong focal energy account")
                    energy[event["phase"]] += event["delta_e"]
                observed = False
                for event in row["trace"]:
                    phase = event["phase"]
                    if phase == "food_gate_failed":
                        gates[phase] += 1
                    if phase != "reproduction_gates":
                        continue
                    require(not observed, "Duplicate gate check")
                    observed = True
                    gates["evaluated"] += 1
                    failed = {key for key, passed in event.items() if key.endswith("_ok") and not passed}
                    blocks.update(failed)
                    gates["ready_all"] += not failed
                    gates["energy_ready"] += not failed.intersection({"energy_ok", "transfer_e_ok"})
                    gates["only_energy_blocked"] += bool(failed) and failed <= {"energy_ok", "transfer_e_ok"}
                if row["tick"] > start and row["focal"] is not None and not observed:
                    gates["present_without_gate_check"] += 1
            else:
                audit.accept(row)
                if kind in EVENTS and (kind != "snapshot" or row["tick"] % 1000 == 0):
                    trajectory.append(row)
                if kind == "birth" and row["parent"] == ident and row["tick"] > start:
                    direct.append(row["tick"])
                if kind in ("removal", "survivor") and row["id"] == ident:
                    terminal = row
    audit.finish()
    require(intervention is not None and terminal is not None, "Missing intervention or terminal")
    require([r["tick"] for r in probes] == list(range(start, end + 1)), "Incomplete focal trace")
    initial = probes[0]["focal"]
    require(initial["hp"] > 0 and terminal["births"] - initial["births"] == len(direct),
            "Focal state or births mismatch")
    upkeep = terminal["upkeep"] - initial["upkeep"]
    require(abs(energy["resistance_upkeep"] + upkeep) < 1e-8, "Unreconciled upkeep")
    if intervention["mode"] == "exempt":
        require(upkeep == 0, "Exempt individual paid upkeep")
    result = dict(intervention=intervention, terminal=terminal, initial=initial, direct_birth_ticks=direct,
                  gate_counts=dict(gates), blocked_by=dict(blocks), energy_by_phase=dict(energy),
                  plants_min=min(r["plants"] for r in probes), plants_max=max(r["plants"] for r in probes),
                  dose=terminal["dose"]-initial["dose"], damage=terminal["damage"]-initial["damage"],
                  upkeep=upkeep, log_sha256=sha256(path), max_n_error=audit.complete["max_n_error"],
                  max_e_error=audit.complete["max_e_error"])
    return result, trajectory


def main(directory, baseline, observed):
    plan = json.loads((directory / "plan.json").read_text(encoding="utf-8"))
    result = {}
    for case in plan["cases"]:
        name = case["name"]
        config = json.loads((baseline / f"{name}.input.json").read_text(encoding="utf-8"))
        source = observed / f"{name}.jsonl"
        require(sha256(source) == case["source_sha256"], "Baseline source changed")
        result[name] = {}
        states = []
        for mode in ("paid", "exempt"):
            row, trajectory = summarize(directory / f"{name}_{mode}.jsonl", config, case)
            require(row["intervention"]["mode"] == mode, "Intervention mode mismatch")
            states.append(row["intervention"]["state"])
            if mode == "paid":
                require(all(a == b for a, b in zip_longest(trajectory, baseline_events(source, case["end"]))),
                        "Paid trajectory differs from the archived run")
            result[name][mode] = row
        require(states[0] == states[1], "Worlds differ before intervention")
    with (directory / "assessment.json").open("x", encoding="utf-8") as stream:
        json.dump(result, stream, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("observed", type=Path)
    args = parser.parse_args()
    main(args.directory, args.baseline, args.observed)
