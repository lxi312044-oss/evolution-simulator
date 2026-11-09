"""Audit capacity-blocked reproduction without simulating untested food checks."""
import argparse
import json
from collections import Counter
from itertools import zip_longest
from pathlib import Path

from analyze_reproduction_health import audit_run, load, read_rows
from run_confirmation import require
from run_metadata import sha256


def original_trace(rows):
    for row in rows:
        if row["kind"] != "health" or row["phase"] == "swap_remove_move":
            continue
        row = {k: v for k, v in row.items() if k != "seq"}
        if row["phase"] == "capacity_blocked":
            row["details"] = None
        if row["phase"] == "reproduction_gates":
            row["details"] = {k: v for k, v in row["details"].items()
                              if k not in {"index", "entry_herbivores"}}
        yield row


def summarize(events, births, case, cap):
    by_individual = {tuple(i): Counter() for i in case["ids"]}
    patterns = Counter()
    tick_rows = {}
    births_by_tick = Counter(row["tick"] for row in births)
    index = {}
    order = []
    moves = []
    for event in events:
        if not case["start"] < event["tick"] <= case["end"]:
            continue
        phase = event["phase"]
        if phase not in {"capacity_blocked", "reproduction_gates", "swap_remove_move"}:
            continue
        ident = tuple(event["id"])
        tick, d = event["tick"], event["details"]
        row = by_individual[ident]
        if phase == "swap_remove_move":
            require(d["from"] > d["to"], "Unexpected swap_remove direction")
            if ident in index:
                require(index[ident] == d["from"], "Discontinuous parent array order")
            index[ident] = d["to"]
            moves.append(dict(tick=tick, id=list(ident), **d))
            row["moves"] += 1
            continue
        if ident in index:
            require(index[ident] == d["index"], "Unrecorded change in parent index")
        index[ident] = d["index"]
        row[phase] += 1
        order.append(dict(tick=tick, id=list(ident), index=d["index"],
                          parent_count=d["entry_herbivores"], outcome=phase))
        if phase != "capacity_blocked":
            continue
        require(d["capacity"] == cap and d["plants"] + d["herbivores"] >= cap,
                "Recorded cut did not reach capacity")
        require(d["first_unprocessed"] <= d["index"] < d["entry_herbivores"],
                "Invalid cutoff index")
        require(d["earlier_births"] == d["herbivores"] - d["entry_herbivores"] == births_by_tick[tick],
                "Earlier births do not reconcile")
        full_at_entry = d["plants"] + d["entry_herbivores"] >= cap
        require(full_at_entry == (d["first_unprocessed"] == 0), "Inconsistent entry occupancy")
        require(not d["food_evaluated"], "A blocked individual received a food draw")
        flags = {k: v for k, v in d.items() if k.endswith("_ok")}
        require(len(flags) == 8 and all(isinstance(v, bool) for v in flags.values()),
                "Missing deterministic conditions")
        failed = "+".join(sorted(k for k, v in flags.items() if not v)) or "all_basic_pass"
        row[failed] += 1
        patterns[failed] += 1
        row["full_at_entry" if full_at_entry else "filled_by_earlier_births"] += 1
        shared = {k: d[k] for k in ("plants", "herbivores", "carnivores", "capacity",
                                   "entry_herbivores", "first_unprocessed", "earlier_births")}
        shared["full_at_entry"] = full_at_entry
        require(tick_rows.setdefault(tick, shared) == shared, "Inconsistent capacity composition")
    return dict(individuals=[dict(id=list(i), counts=dict(c)) for i, c in by_individual.items()],
                failure_patterns=dict(patterns), capacity_ticks=tick_rows,
                swap_moves=moves, order=order)


def main(directory, previous):
    plan = load(directory / "plan.json")
    old = load(previous / "assessment.json")
    results = {}
    for name, case in plan["cases"].items():
        if not case["ids"]:
            results[name] = dict(arm=case["arm"], individuals=[], failure_patterns={},
                                 capacity_ticks={}, swap_moves=[], order=[])
            continue
        config_path = Path(case["config"])
        require(sha256(config_path) == case["input_sha256"], "Configuration changed")
        config = load(config_path)
        source = Path(case["source"])
        path = directory / f"{name}.jsonl"
        earlier = previous / f"{name}.jsonl"
        require(sha256(earlier) == old[name]["quality"]["log_sha256"], "Earlier health trace changed")
        audit, events, births, states, quality = audit_run(path, source, config, case)
        require(sorted(s["id"] for s in states) == case["ids"], "Trough cohort changed")
        require(all(a == b for a, b in zip_longest(original_trace(read_rows(path)),
                                                   original_trace(read_rows(earlier)))),
                "Earlier observed health trajectory changed")
        result = summarize(events, births, case, config["world"]["max_living_entities"])
        require(sum(i["counts"].get("capacity_blocked", 0) for i in result["individuals"]) ==
                old[name]["summary"]["capacity_blocked"], "Prior capacity count changed")
        results[name] = dict(arm=case["arm"], quality=quality, **result)
        counts = Counter()
        for individual in result["individuals"]:
            counts.update(individual["counts"])
        print(name, len(result["capacity_ticks"]), dict(counts), flush=True)
    with (directory / "assessment.json").open("x", encoding="utf-8") as stream:
        json.dump(results, stream, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("previous", type=Path)
    args = parser.parse_args()
    main(args.directory, args.previous)
