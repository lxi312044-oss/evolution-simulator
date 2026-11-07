"""Bounded, preregistered focal-resistance pilot. No standard model changes."""
import argparse
from collections import defaultdict
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import random
import statistics
import subprocess
import time

from run_metadata import record, sha256, source_hashes
from analyze_phase0759 import require, write_csv

ROOT = Path(__file__).resolve().parents[1]
EXAMPLE = "resistance_focal_pilot"
ENVIRONMENTS = ("A", "B", "off")


def matrix(seeds):
    return [(seed, env, ty, level, defense)
            for seed in seeds for env in ENVIRONMENTS for ty in (0, 1)
            for level in ("low", "high") for defense in ("off", "on")]


def name_for(case):
    seed, env, ty, level, defense = case
    return f"seed{seed}_{env}_type{ty}_{level}_{defense}"


def digest_json(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def validate_rows(rows, seeds, ticks, interval):
    expected = set(matrix(seeds))
    cases = {}
    for row in rows:
        case = tuple(row["case"])
        require(case in expected and case not in cases, "Unexpected/duplicate pilot case")
        seed, env, ty, level, defense = case
        r = 0.2 if level == "low" else 0.8
        require(row["seed"] == seed and row["environment"] == env and row["toxin_type"] == ty,
                "Pilot identity mismatch")
        require(abs(row["resistance"]-r) < 1e-7 and row["defense"] == (0.8 if defense == "on" else 0),
                "Pilot intervention mismatch")
        require(row["ticks"] == ticks and row["completed_tick"] == ticks and row["interval"] == interval,
                "Incomplete/wrong observation horizon")
        config = row["effective_config"]
        t = config["toxin"]
        require(config["seed"] == seed and t["enabled"] == (env != "off"), "Effective environment mismatch")
        require(t["initial_type0_fraction"] == (0 if env == "B" else 1) and
                t["plant_toxin_type_count"] == 2 and t["plant_toxin_type_mutation_probability"] == 0 and
                t["resistance_mutation_probability"] == 0 and t["initial_resistance"] == 0.5 and
                t["resistance_enabled"] and t["herbivore_multi_toxin_enabled"] and
                t["multi_type_resistance_upkeep_multiplier"] == 0.5 and
                t["additional_type_resistance_upkeep_multiplier"] == 1 and
                t["resistance_effectiveness"] == row["defense"], "Effective diagnostic config mismatch")
        initial, outcome = row["initial"], row["outcome"]
        require(initial["tick"] == 0 and initial["alive"] and initial["births"] == 0, "Invalid founder observation")
        require(0 < outcome["tick"] <= ticks and
                (not outcome["alive"] or outcome["tick"] == ticks), "Invalid focal terminal")
        require(outcome["births"] >= 0 and isinstance(outcome["births"], int), "Invalid offspring count")
        require(outcome["trigger"] in (("none",) if outcome["alive"] else ("hp_nonpositive", "age_limit")),
                "Invalid focal removal trigger")
        snapshots = row["checkpoints"]
        expected_ticks = [n for n in range(0, ticks+1, interval)
                          if outcome["alive"] or n < outcome["tick"]]
        if outcome["alive"] and ticks not in expected_ticks:
            expected_ticks.append(ticks)
        require([s["tick"] for s in snapshots] == expected_ticks, "Focal checkpoint coverage mismatch")
        for sample in snapshots + [outcome]:
            for field in ("hp", "damage", "prevented", "cost", "dose_a", "dose_b"):
                require(isinstance(sample[field], (int, float)) and math.isfinite(sample[field]),
                        "Nonfinite pilot observation")
            require(all(sample[k] >= 0 for k in ("damage", "cost", "dose_a", "dose_b")),
                    "Negative damage/cost/exposure")
            require(abs(sample["resistance_a"]-(r if ty == 0 else 0.5)) < 1e-7 and
                    abs(sample["resistance_b"]-(r if ty == 1 else 0.5)) < 1e-7,
                    "Focal heritable resistance changed unexpectedly")
            if env == "off":
                require(sample["dose_a"] == sample["dose_b"] == sample["damage"] == 0, "Exposure in toxin-OFF")
            elif env == "A":
                require(sample["dose_b"] == 0, "Type B contamination")
            else:
                require(sample["dose_a"] == 0, "Type A contamination")
        for field in ("max_n_error", "max_e_error"):
            require(math.isfinite(row[field]) and row[field] >= 0, "Invalid resource error")
        require(row["max_n_error"] <= 1e-3, "Nutrient conservation error")
        cases[case] = row
    require(set(cases) == expected, "Missing pilot cases")
    backgrounds = defaultdict(set)
    initials = defaultdict(set)
    for case, row in cases.items():
        seed, env, ty, level, defense = case
        backgrounds[(seed, env)].add(row["initial_background_sha256"])
        # Only the focal resistance fields are allowed to differ in the initial observation.
        initial = {k: v for k, v in row["initial"].items() if k not in ("resistance_a", "resistance_b")}
        initials[(seed, env)].add(digest_json(initial))
    require(all(len(v) == 1 for v in backgrounds.values()), "Initial world backgrounds differ")
    require(all(len(v) == 1 for v in initials.values()), "Initial focal nonresistance state differs")
    for seed in seeds:
        for ty in (0, 1):
            for level in ("low", "high"):
                a = cases[(seed, "off", ty, level, "on")]
                b = cases[(seed, "off", ty, level, "off")]
                require(all(a[k] == b[k] for k in ("initial", "outcome", "checkpoints", "final_ecology",
                                                  "max_n_error", "max_e_error")),
                        "Toxin-OFF defense toggle changed trajectory")
    return cases


def effects(cases, seeds):
    contrasts, interactions = [], []
    for seed in seeds:
        for env in ENVIRONMENTS:
            for ty in (0, 1):
                arms = {}
                for defense in ("on", "off"):
                    low = cases[(seed, env, ty, "low", defense)]["outcome"]
                    high = cases[(seed, env, ty, "high", defense)]["outcome"]
                    row = dict(seed=seed, environment=env, toxin_type=ty, defense=defense)
                    for key in ("births", "alive", "damage", "cost", "dose_a", "dose_b"):
                        row[key + "_high_minus_low"] = high[key]-low[key]
                    row.update(births_low=low["births"], births_high=high["births"])
                    contrasts.append(row)
                    arms[defense] = row
                interactions.append(dict(seed=seed, environment=env, toxin_type=ty,
                    births_interaction=arms["on"]["births_high_minus_low"]-arms["off"]["births_high_minus_low"],
                    survival_interaction=arms["on"]["alive_high_minus_low"]-arms["off"]["alive_high_minus_low"],
                    births_high_minus_low_on=arms["on"]["births_high_minus_low"],
                    births_high_minus_low_off=arms["off"]["births_high_minus_low"]))
    return contrasts, interactions


def summarize_interactions(interactions):
    groups = defaultdict(list)
    for row in interactions:
        groups[(row["environment"], row["toxin_type"])].append(row["births_interaction"])
    result = []
    rng = random.Random(750907)
    for (env, ty), values in sorted(groups.items()):
        n = len(values)
        sd = statistics.stdev(values) if n > 1 else 0
        if sd > 0:
            samples = sorted(statistics.mean(rng.choices(values, k=n)) for _ in range(5000))
            lower, upper = samples[124], samples[4874]
            rough_n = max(8, math.ceil(((1.96+0.84)*sd/0.2)**2))
        else:
            lower = upper = rough_n = ""
        result.append(dict(environment=env, toxin_type=ty, seeds=n,
            mean_births_interaction=statistics.mean(values), sd_between_seeds=sd,
            positive=sum(v > 0 for v in values), negative=sum(v < 0 for v in values),
            zero=sum(v == 0 for v in values), exploratory_ci_low=lower, exploratory_ci_high=upper,
            rough_normal_n_for_0_2_births=rough_n,
            uncertainty="exploratory_seed_bootstrap" if sd else "not_estimable_zero_sample_variation"))
    return result


def analyze(root, protocol):
    rows = []
    for case in matrix(protocol["seeds"]):
        name = name_for(case)
        path = root / f"{name}.result.json"
        meta = json.loads((root / f"{name}.metadata.json").read_text())
        require(meta["status"] == "completed" and meta["result_sha256"] == sha256(path), "Incomplete/modified result")
        require(meta["inputs"]["config_sha256"] == sha256(root / f"seed{case[0]}.input.json") and
                meta["inputs"]["ticks"] == protocol["ticks"] and meta["inputs"]["interval"] == protocol["interval"] and
                meta["example"] == EXAMPLE, "Input config/horizon/example provenance mismatch")
        require(meta["inputs"]["source_sha256"] == protocol["source_sha256"] and
                meta["inputs"]["binary_sha256"] == protocol["binary_sha256"], "Run provenance mismatch")
        rows.append(json.loads(path.read_text()))
    cases = validate_rows(rows, protocol["seeds"], protocol["ticks"], protocol["interval"])
    contrasts, interactions = effects(cases, protocol["seeds"])
    summary = summarize_interactions(interactions)
    write_csv(root / "resistance_contrasts.csv", contrasts)
    write_csv(root / "defense_interactions.csv", interactions)
    write_csv(root / "pilot_summary.csv", summary)
    quality = dict(passed=True, runs=len(rows), independent_seeds=len(protocol["seeds"]),
                   ticks=protocol["ticks"], pilot_only=True, initial_backgrounds_verified=True,
                   toxin_off_defense_equivalence=True,
                   source_manifest_sha256=digest_json(protocol["source_sha256"]),
                   binary_sha256=protocol["binary_sha256"],
                   input_sha256={p.name: sha256(p) for p in sorted(root.glob("*.result.json"))},
                   limits=["One founder per world; initial ages vary across seeds but match within pairs.",
                           "Defense toggle affects the whole world; ecological feedback remains.",
                           "No spontaneous evolution claim; mutations disabled for diagnostic traits.",
                           "Eight pilot seeds are not independent confirmation; zero variance does not prove no effect.",
                           "Damage/exposure/cost at terminal depend on survival duration."])
    (root / "quality.json").write_text(json.dumps(quality, indent=2), encoding="utf-8")
    return quality, summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--smoke", action="store_true")
    args = parser.parse_args()
    root = args.out.resolve()
    require(not root.exists() or not any(root.iterdir()), "Output directory must be new/empty")
    root.mkdir(parents=True, exist_ok=True)
    subprocess.run(["cargo", "build", "--locked", "--release", "-p", "sim-core", "--example", EXAMPLE],
                   cwd=ROOT, check=True)
    cargo = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT))
    binary = Path(cargo["target_directory"]) / "release/examples" / (EXAMPLE + ".exe")
    base_path = ROOT / "docs/phase07/phase0753-pilot-seed12345.json"
    base = json.loads(base_path.read_text(encoding="utf-8-sig"))
    seeds = list(range(23451, 23453 if args.smoke else 23459))
    ticks, interval = (100, 100) if args.smoke else (3000, 100)
    protocol = dict(schema_version=1, seeds=seeds, ticks=ticks, interval=interval, smoke=args.smoke,
                    source_sha256=source_hashes(ROOT), binary_sha256=sha256(binary),
                    base_config_sha256=sha256(base_path), runner_sha256=sha256(__file__),
                    planned_cases=[list(c) for c in matrix(seeds)],
                    started_at=datetime.now(timezone.utc).isoformat(), status="running")
    (root / "protocol.json").write_text(json.dumps(protocol, indent=2), encoding="utf-8")
    started = time.monotonic()
    try:
        for index, case in enumerate(matrix(seeds), 1):
            require(time.monotonic()-started < 1800, "Pilot exceeded 30-minute limit")
            seed, env, ty, level, defense = case
            name = name_for(case)
            config = dict(base, seed=seed)
            config_path = root / f"seed{seed}.input.json"
            if not config_path.exists():
                config_path.write_text(json.dumps(config, indent=2), encoding="utf-8")
            meta_path = root / f"{name}.metadata.json"
            record("begin", ROOT, config_path, binary, meta_path, ticks, interval, EXAMPLE)
            command = [str(binary), str(config_path), env, str(ty),
                       "0.2" if level == "low" else "0.8", "0.8" if defense == "on" else "0",
                       str(ticks), str(interval)]
            run = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, check=True, timeout=120)
            result = json.loads(run.stdout)
            result["case"] = list(case)
            result["initial_background_sha256"] = digest_json(result.pop("initial_background"))
            result_path = root / f"{name}.result.json"
            result_path.write_text(json.dumps(result, indent=2), encoding="utf-8")
            meta = record("complete", ROOT, config_path, binary, meta_path, ticks, interval, EXAMPLE)
            meta["result_sha256"] = sha256(result_path)
            meta_path.write_text(json.dumps(meta, indent=2), encoding="utf-8")
            if index % 8 == 0:
                print(f"Completed {index}/{len(matrix(seeds))}", flush=True)
        require(source_hashes(ROOT) == protocol["source_sha256"] and sha256(binary) == protocol["binary_sha256"]
                and sha256(__file__) == protocol["runner_sha256"], "Inputs changed during pilot")
        quality, summary = analyze(root, protocol)
        protocol.update(status="completed", completed_at=datetime.now(timezone.utc).isoformat())
        (root / "protocol.json").write_text(json.dumps(protocol, indent=2), encoding="utf-8")
        print(json.dumps(dict(passed=quality["passed"], runs=quality["runs"], summary=summary), indent=2))
    except Exception as error:
        (root / "quality.json").write_text(json.dumps(dict(passed=False, error=str(error)), indent=2), encoding="utf-8")
        raise


if __name__ == "__main__":
    main()
