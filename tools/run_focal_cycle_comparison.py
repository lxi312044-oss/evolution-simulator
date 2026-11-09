"""Run and audit the focal resistance comparison in baseline-defined cycle windows."""
import argparse
import json
import subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from analyze_focal_toxic_fraction import summarize
from run_confirmation import require
from run_longterm_resistance import interval
from run_metadata import sha256

ROOT = Path(__file__).resolve().parents[1]
WORKER = ROOT / "tools/focal_upkeep_probe.mjs"


def run_case(directory, runtime, case):
    seed, fraction, resistance, first, second = case
    name = f"{seed}_{fraction}_{resistance}"
    with (directory / f"{name}.jsonl").open("xb") as stream:
        subprocess.run(
            ["node", str(WORKER), str(directory / f"{name}.input.json"), str(runtime),
             "paid", str(resistance), str(second)], stdout=stream, check=True, timeout=600,
        )
    print(f"{name} completed at {second}", flush=True)


def audit(directory):
    plan = json.loads((directory / "plan.json").read_text(encoding="utf-8"))
    results, initial_states = {}, {}
    for seed, fraction, resistance, first, second in plan["cases"]:
        name = f"{seed}_{fraction}_{resistance}"
        config = json.loads((directory / f"{name}.input.json").read_text(encoding="utf-8"))
        require((config["seed"], config["toxin"]["initial_toxic_plant_fraction"]) ==
                (seed, fraction), f"Unexpected configuration: {name}")
        result, state = summarize(directory / f"{name}.jsonl", config, second, first)
        baseline = initial_states.setdefault((seed, fraction), state)
        require(state == baseline, f"Initial state mismatch: {name}")
        require(abs(result["terminal"]["resistance"] - resistance) < 1e-7,
                f"Resistance mismatch: {name}")
        results[name] = result
    metrics = ("direct_births", "descendant_births", "first_cycle_present",
               "second_cycle_births", "final_present")
    seeds = sorted({case[0] for case in plan["cases"]})
    report = {}
    for metric in metrics:
        paired = {
            fraction: [results[f"{seed}_{fraction}_0.8"][metric] -
                       results[f"{seed}_{fraction}_0.2"][metric] for seed in seeds]
            for fraction in (0.1, 0.9)
        }
        report[metric] = {str(fraction): dict(**interval(values), by_seed=dict(zip(seeds, values)))
                          for fraction, values in paired.items()}
        interaction = [rich - poor for poor, rich in zip(paired[0.1], paired[0.9])]
        report[metric]["interaction"] = dict(**interval(interaction),
                                              by_seed=dict(zip(seeds, interaction)))
    assessment = directory / "assessment.json"
    contents = json.dumps(dict(runs=results, contrasts=report), indent=2)
    if assessment.exists():
        require(json.loads(assessment.read_text(encoding="utf-8")) == json.loads(contents),
                "Saved assessment differs from the audited logs")
    else:
        with assessment.open("x", encoding="utf-8") as stream:
            stream.write(contents)


def run(directory, baseline, runtime):
    plan = json.loads((baseline / "plan.json").read_text(encoding="utf-8"))
    windows = json.loads((baseline / "assessment.json").read_text(encoding="utf-8"))
    require(sha256(runtime / "sim_wasm_bg.wasm") == plan["wasm_sha256"], "WASM mismatch")
    directory.mkdir()
    cases = []
    for seed, fraction in plan["cases"]:
        name = f"{seed}_{fraction}"
        require(sha256(baseline / f"{name}.jsonl") == windows[name]["log_sha256"],
                f"Baseline log changed: {name}")
        cycles = windows[name]["cycles"]
        require(len(cycles) == 2, f"Unresolved baseline: {name}")
        first, second = [c["end_tick"] for c in cycles]
        require(0 < first < second <= plan["cap"], "Invalid baseline boundaries")
        for resistance in (0.2, 0.8):
            cases.append((seed, fraction, resistance, first, second))
            (directory / f"{name}_{resistance}.input.json").write_bytes(
                (baseline / f"{name}.input.json").read_bytes()
            )
    with (directory / "plan.json").open("x", encoding="utf-8") as stream:
        json.dump(dict(cases=cases, primary="descendant_births",
                       baseline_sha256=sha256(baseline / "assessment.json"),
                       wasm_sha256=plan["wasm_sha256"],
                       worker_sha256=sha256(WORKER)), stream, indent=2)
    with ThreadPoolExecutor(max_workers=4) as pool:
        list(pool.map(lambda case: run_case(directory, runtime, case), cases))
    audit(directory)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--runtime", type=Path)
    args = parser.parse_args()
    if args.baseline and args.runtime:
        run(args.directory, args.baseline, args.runtime)
    elif args.baseline or args.runtime:
        parser.error("--baseline and --runtime must be supplied together")
    else:
        audit(args.directory)
