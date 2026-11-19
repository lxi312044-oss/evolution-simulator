"""Compare population resistance with toxin effects enabled and disabled."""
import argparse
import json
import random
from pathlib import Path
from statistics import mean

from analyze_focal_cycle_baselines import detect
from run_confirmation import require
from run_longterm_resistance import Audit, resistance_bin
from run_metadata import sha256


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def replay(path, config, plan):
    audit = Audit(config, plan["cap"])
    samples = {}
    births = removals = inherited = 0
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line)
            kind = row["kind"]
            if kind == "population":
                tick = row["tick"]
                require(tick not in samples and sum(row["counts"]) == row["living"], "Population sample mismatch")
                if audit.samples[-1]["tick"] == tick:
                    last = audit.samples[-1]
                    require(last["positive"] == row["living"] and last["plants"] == row["plants"]
                            and last["positive_high_fraction"] ==
                            (row["counts"][2] / row["living"] if row["living"] else None),
                            "Population snapshot mismatch")
                samples[tick] = dict(row, high_count=row["counts"][2],
                                     high_fraction=row["counts"][2] / row["living"] if row["living"] else None,
                                     high_births=births, high_removals=removals,
                                     high_births_from_high=inherited)
                continue
            audit.accept(row)
            if kind == "birth" and resistance_bin(row["resistance"]) == 2:
                births += 1
                inherited += resistance_bin(row["parent_resistance"]) == 2
            if kind == "removal" and resistance_bin(row["resistance"]) == 2:
                removals += 1
    audit.finish()
    require(list(samples) == list(range(0, plan["cap"] + 1, plan["interval"])), "Missing population checkpoints")
    return samples, audit.initial["initial_state"], {
        "max_n_error": audit.complete["max_n_error"],
        "max_e_error": audit.complete["max_e_error"],
        "log_sha256": sha256(path),
    }


def window(samples, start, end):
    before, after = samples[start], samples[end]
    deltas = {k: after[k] - before[k] if before[k] is not None and after[k] is not None else None
              for k in ("high_count", "high_fraction", "high_births", "high_removals", "high_births_from_high")}
    return dict(start=before, end=after, change=deltas)


def summarize_pairs(pairs, seed):
    report = {}
    for metric in ("high_count", "high_fraction", "high_births"):
        differences = {name: values["on"]["change"][metric] - values["off"]["change"][metric]
                       for name, values in pairs.items()
                       if values["on"]["change"][metric] is not None and values["off"]["change"][metric] is not None}
        values = list(differences.values())
        rng = random.Random(seed)
        draws = sorted(mean(rng.choices(values, k=len(values))) for _ in range(4000)) if values else []
        report[metric] = dict(n=len(values), mean=mean(values) if values else None,
                              ci95=[draws[99], draws[3899]] if len(values) > 1 else None,
                              by_seed=differences)
    return report


def main(control, observed, baseline):
    plan = load(control / "plan.json")
    windows = load(baseline / "assessment.json")
    originals = load(observed / "assessment.json")
    matched = {"same_tick": {}, "same_phase": {}}
    runs = {}
    for seed, fraction in plan["cases"]:
        name = f"{seed}_{fraction}"
        on_config = load(baseline / f"{name}.input.json")
        off_config = load(control / f"{name}.input.json")
        require(on_config["toxin"]["enabled"] and not off_config["toxin"]["enabled"], "Incorrect toxin arms")
        require(off_config == {**on_config, "toxin": {**on_config["toxin"], "enabled": False}},
                "Configuration differs beyond toxin.enabled")
        on, initial_on, on_quality = replay(observed / f"{name}.jsonl", on_config, plan)
        off, initial_off, off_quality = replay(control / f"{name}.jsonl", off_config, plan)
        require(on_quality["log_sha256"] == originals[name]["log_sha256"], "Changed toxin-on log")
        require(initial_on == initial_off, "Initial ecology differs")
        on_cycle = windows[name]["cycles"][1]
        off_cycles = detect(list(off.values()), plan, plan["window"])
        start, end = on_cycle["trough_tick"], on_cycle["end_tick"]
        on_window = window(on, start, end)
        matched["same_tick"][name] = dict(on=on_window, off=window(off, start, end))
        if len(off_cycles) == 2:
            off_cycle = off_cycles[1]
            matched["same_phase"][name] = dict(
                on=on_window, off=window(off, off_cycle["trough_tick"], off_cycle["end_tick"])
            )
        runs[name] = dict(off_cycles=off_cycles, on_quality=on_quality, off_quality=off_quality)
        print(f"{name} passed, OFF boundaries {[c['end_tick'] for c in off_cycles]}", flush=True)
    report = {key: summarize_pairs(pairs, plan["bootstrap_seed"]) for key, pairs in matched.items()}
    with (control / "assessment.json").open("x", encoding="utf-8") as stream:
        json.dump(dict(runs=runs, pairs=matched, contrasts=report), stream, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("control", type=Path)
    parser.add_argument("observed", type=Path)
    parser.add_argument("baseline", type=Path)
    args = parser.parse_args()
    main(args.control, args.observed, args.baseline)
