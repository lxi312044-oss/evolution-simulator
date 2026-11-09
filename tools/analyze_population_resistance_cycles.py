"""Audit an untreated replay and summarize resistance at fixed ecological phases."""
import argparse
import json
from collections import Counter
from pathlib import Path

from run_confirmation import require
from run_longterm_resistance import Audit, resistance_bin
from run_metadata import sha256


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def summarize(path, config, baseline, cycles, cap):
    audit = Audit(config, cap)
    expected = {row["tick"]: row for row in baseline}
    marks = {0: "initial"}
    for index, cycle in enumerate(cycles, 1):
        for stage, field in (("peak", "peak_tick"), ("trough", "trough_tick"), ("recovery", "end_tick")):
            marks[cycle[field]] = f"cycle{index}_{stage}"
    births, removals, high_from_high = Counter(), Counter(), 0
    phases = {}
    seen = []
    prior_present = [0, 0, 0]
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line)
            kind = row["kind"]
            if kind != "population":
                audit.accept(row)
                if kind == "birth":
                    group = resistance_bin(row["resistance"])
                    births[group] += 1
                    high_from_high += group == 2 and resistance_bin(row["parent_resistance"]) == 2
                elif kind == "removal":
                    removals[resistance_bin(row["resistance"])] += 1
                continue
            tick = row["tick"]
            seen.append(tick)
            require(all(row[k] == expected[tick][k] for k in ("living", "plants")),
                    f"Baseline trajectory mismatch at {path.name}:{tick}")
            counts = row["counts"]
            present = Counter(resistance_bin(audit.life[i]["r"]) for i in audit.live)
            require(sum(counts) == row["living"] and all(0 <= n <= present[i] for i, n in enumerate(counts)),
                    "Positive-HP distribution mismatch")
            if audit.samples[-1]["tick"] == tick:
                checkpoint = audit.samples[-1]
                require(checkpoint["positive"] == row["living"] and
                        checkpoint["positive_high_fraction"] ==
                        (counts[2] / row["living"] if row["living"] else None), "Snapshot mismatch")
            if tick in marks:
                if tick:
                    require(all(present[i] == prior_present[i] + births[i] - removals[i]
                                for i in range(3)), "Phase population ledger mismatch")
                phases[marks[tick]] = dict(
                    tick=tick, living=row["living"], plants=row["plants"], counts=counts,
                    high_fraction=counts[2] / row["living"] if row["living"] else None,
                    mean=row["mean"], present=[present[i] for i in range(3)],
                    births=[births[i] for i in range(3)], removals=[removals[i] for i in range(3)],
                    high_births_from_high=high_from_high,
                )
                prior_present = [present[i] for i in range(3)]
                births.clear()
                removals.clear()
                high_from_high = 0
    audit.finish()
    require(seen == list(expected), "Missing or reordered population checkpoints")
    require(len(phases) == 7, "Missing ecological phase")
    return dict(phases=phases, checkpoints=len(seen), log_sha256=sha256(path),
                max_n_error=audit.complete["max_n_error"], max_e_error=audit.complete["max_e_error"])


def main(directory, baseline):
    plan = load(directory / "plan.json")
    windows = load(baseline / "assessment.json")
    results = {}
    for seed, fraction in plan["cases"]:
        name = f"{seed}_{fraction}"
        old = baseline / f"{name}.jsonl"
        require(sha256(old) == windows[name]["log_sha256"], "Baseline log changed")
        with old.open(encoding="utf-8") as stream:
            previous = [json.loads(line) for line in stream]
        results[name] = summarize(directory / f"{name}.jsonl",
                                  load(baseline / f"{name}.input.json"), previous,
                                  windows[name]["cycles"], plan["cap"])
        print(f"{name} passed", flush=True)
    with (directory / "assessment.json").open("x", encoding="utf-8") as stream:
        json.dump(results, stream, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("baseline", type=Path)
    args = parser.parse_args()
    main(args.directory, args.baseline)
