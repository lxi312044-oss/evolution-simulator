"""Run the fixed inherited-color comparison after the archived-state compatibility audit."""
import gzip
import json
import os
import subprocess
from pathlib import Path

from run_stage4_carnivore_coefficient import LIMIT, OVERRIDE, SEEDS, analyze, sha256

REFERENCE = Path("config/stage4-inherited-plant-colors.json")
CANDIDATE = Path("config/stage4-inherited-predator-candidate.json")
ARCHIVE = Path("results/stage4-inherited-plant-colors")
OUT = Path("results/stage4-inherited-predator")


def summarize_predators(run, clearance, threshold):
    samples = run["samples"]
    population = run["combined_population"]
    rows = [
        {**sample, **population_at_tick, "animal_toxin": sample["toxin"]}
        for sample, population_at_tick in zip(samples, population, strict=True)
    ]
    predator_run = {
        "trace": run.pop("predator_trace"), "rows": rows, "first_zero": run["first_zero"],
        "plant_resources": [[s["tick"], s["plant_body_n"], s["plant_reserve_e"]] for s in samples],
        "max_n_error": run["max_n_error"], "max_e_error": run["max_e_error"],
        "max_sampled_toxin_error": run["max_toxin_error"], "max_population": run["max_population"],
        "seconds": run["seconds"], "terminal_state_hash": None,
    }
    predators, decisions = analyze(predator_run, clearance, threshold)
    del predators["rows"]
    del predators["terminal_state_hash"]
    return predators, decisions


def main():
    binary = Path("target/release/examples") / (
        "stage4_herbivore_learning.exe" if os.name == "nt" else "stage4_herbivore_learning"
    )
    baseline = json.loads(REFERENCE.read_text())
    candidate = json.loads(CANDIDATE.read_text())
    assert candidate["animal_toxin"].pop(OVERRIDE) == 0.50
    assert candidate["animal_toxin"]["carnivore_damage_threshold"] == 0.05
    assert baseline["animal_toxin"]["carnivore_damage_threshold"] == 0.50
    assert baseline["toxin"]["hp_damage_per_excess_unit"] == 0.01
    candidate["animal_toxin"]["carnivore_damage_threshold"] = 0.50
    assert baseline == candidate
    assert baseline["herbivores"]["color_learning_enabled"]
    assert not baseline["herbivores"].get("hungry_color_recheck_enabled", False)
    OUT.mkdir()
    manifest = {"binary_sha256": sha256(binary), "conditions": []}
    used = 0
    for seed in SEEDS:
        reference_decisions = None
        for label, config, threshold in (("reference", REFERENCE, 0.50),
                                         ("candidate", CANDIDATE, 0.05)):
            row = {"seed": seed, "arm": label, "config_sha256": sha256(config)}
            try:
                process = subprocess.run(
                    [str(binary), str(seed), f"inherited-predator-{label}"],
                    capture_output=True, check=True, timeout=120,
                )
                run = json.loads(process.stdout)
                assert (run["seed"], run["learning"], run["ticks"]) == (seed, True, 6000)
                if label == "reference":
                    with gzip.open(ARCHIVE / f"{seed}-on.json.gz", "rt") as stream:
                        old = json.load(stream)
                    for field in ("samples", "founders", "predator_events",
                                  "inherited_choices", "inherited_total"):
                        assert run[field] == old[field], field
                predator, decisions = summarize_predators(
                    run, baseline["toxin"]["clearance_fraction_per_tick"], threshold
                )
                if label == "reference":
                    reference_decisions = decisions
                elif reference_decisions is not None:
                    predator["first_observed_decision_difference"] = next(
                        ([old, new] for old, new in zip(reference_decisions, decisions)
                         if old != new), None
                    )
                    predator["decision_sequence_lengths"] = [len(reference_decisions), len(decisions)]
                run["predators"] = predator
                data = gzip.compress(json.dumps(run).encode(), mtime=0)
                assert used + len(data) <= LIMIT - 16384, "persistent output limit"
                path = OUT / f"{seed}-{label}.json.gz"
                with path.open("xb") as stream:
                    stream.write(data)
                used += len(data)
                row.update(file=path.name, sha256=sha256(path), bytes=len(data), seconds=run["seconds"])
            except (AssertionError, subprocess.SubprocessError, ValueError) as error:
                stderr = getattr(error, "stderr", None) or b""
                row.update(error=repr(error), stderr=stderr.decode(errors="replace")[-1200:])
            manifest["conditions"].append(row)
            print(json.dumps(row), flush=True)
    with (OUT / "manifest.json").open("x", encoding="utf-8") as stream:
        json.dump(manifest, stream, indent=2)
    if any("error" in row for row in manifest["conditions"]):
        raise SystemExit("fixed comparison incomplete")


if __name__ == "__main__":
    main()
