"""Fixed carnivore damage-coefficient comparison with archived-reference and toxin audits."""
import gzip
import hashlib
import json
import math
import os
import subprocess
from collections import Counter, defaultdict
from pathlib import Path

from observe_stage4_predator_opportunities import BASE, CONFIG, SEEDS, summarize

CANDIDATE = Path("config/stage4-carnivore-coefficient-050.json")
OUT = Path("results/stage4-carnivore-coefficient")
OVERRIDE = "carnivore_hp_damage_per_excess_unit"
LIMIT = 10 * 1024**2


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def analyze(run, clearance, threshold):
    observations = summarize(run, clearance, threshold)
    predators = {}
    first_transfer = {}
    first_memory = {}
    cases = []
    counts = Counter()
    groups = defaultdict(set)
    damage_by_tick = defaultdict(float)
    decisions = []
    evidence = []

    def stage(name, identity):
        counts[name] += 1
        groups[name].add(identity)

    for event in run["trace"]:
        identity = tuple(event["id"])
        tick = event["tick"]
        kind = event["event"]
        record = predators.setdefault(identity, {
            "id": identity, "damage": 0., "max_tick_damage": 0., "removal": None,
            "max_cleared_load": 0., "damage_ticks": 0,
        })
        if kind == "transfer":
            first_transfer.setdefault((identity, event["rgb"]), event)
            evidence.append(event)
        elif kind == "damage":
            record["damage"] += event["damage"]
            damage_by_tick[identity, tick] += event["damage"]
            before = dict(event["memory_before"])
            for color, score in event["memory_after"]:
                key = identity, color
                if score > before.get(color, 0.) and key in first_transfer:
                    first_memory.setdefault(key, event)
            evidence.append(event)
        elif kind == "clearance":
            record["max_cleared_load"] = max(record["max_cleared_load"], event["after"])
        elif kind == "removal":
            record["removal"] = {"tick": tick, "reason": event["reason"]}
            evidence.append(event)
        elif kind == "opportunity":
            target = (event["target"] or {}).get("id")
            decisions.append((tick, identity, target, event["attacked"],
                              event["eaten_n"], event["selection_color_skips"],
                              event["attack_color_rejected"]))
            for encounter in event["learned_ready"]:
                key = identity, encounter["memory_color"]
                if key not in first_memory or first_memory[key]["tick"] >= tick:
                    continue
                prey = encounter["prey"]
                relation = ("original_prey" if encounter["original"] else
                            "other_exact" if encounter["color"] == key[1] else "similar")
                stage("attackable", identity)
                stage(relation, identity)
                if prey in event["selection_color_skips"]:
                    stage("actual_color_skip", identity)
                    outcome = "selection_rejected"
                elif prey == target and event["attack_color_rejected"]:
                    stage("actual_color_skip", identity)
                    outcome = "attack_gate_rejected"
                elif prey == target and event["attacked"]:
                    stage("attacked", identity)
                    if encounter["risk"] > 0.25:
                        stage("attacked_above_full_reserve_tolerance", identity)
                    outcome = "attacked"
                else:
                    outcome = "not_attacked_other_reason"
                    stage(outcome, identity)
                cases.append({
                    "tick": tick, "predator": identity, **encounter,
                    "reserve_e": event["reserve_e"], "tolerance": event["tolerance"],
                    "attack_reserve_e": event["attack_reserve_e"], "selected": prey == target,
                    "outcome": outcome, "eaten_n": event["eaten_n"] if prey == target else 0.,
                    "first_transfer_tick": first_transfer[key]["tick"],
                    "first_memory_tick": first_memory[key]["tick"],
                })
    for (identity, _), damage in damage_by_tick.items():
        row = predators[identity]
        row["max_tick_damage"] = max(row["max_tick_damage"], damage)
        row["damage_ticks"] += 1
    for row in predators.values():
        # Initial HP is 100, so numerical HP and percentage have the same value.
        row["damage_pct_initial_hp"] = row["damage"]
        row["max_tick_pct_initial_hp"] = row["max_tick_damage"]
    final = run["rows"][-1]
    assert len(predators) == run["rows"][0]["counts"][2] + final["carnivore_births"]
    assert sum(row["removal"] is not None for row in predators.values()) == final["carnivore_deaths"]
    assert sum(row["removal"] and row["removal"]["reason"] == "toxin_hp_zero"
               for row in predators.values() if row["removal"]) == final["animal_toxin"]["toxin_deaths"]
    assert math.isclose(sum(row["damage"] for row in predators.values()),
                        sum(x["damage"] for x in evidence if x["event"] == "damage"),
                        abs_tol=1e-12)
    return {
        "observations": observations, "predators": list(predators.values()),
        "decision_counts": counts, "decision_individuals": {k: len(v) for k, v in groups.items()},
        "attackable_cases": cases, "damage_and_transfer_events": evidence,
        "rows": run["rows"], "plant_resources": run["plant_resources"],
        "first_zero": run["first_zero"], "terminal_state_hash": run["terminal_state_hash"],
    }, decisions


def run_condition(binary, config, seed):
    process = subprocess.run(
        [str(binary), str(config), str(seed), "opportunities"],
        capture_output=True, check=True, timeout=120,
    )
    return json.loads(process.stdout)


def main():
    binary = Path("target/release/examples") / (
        "stage4_integrated_baseline.exe" if os.name == "nt" else "stage4_integrated_baseline"
    )
    reference = json.loads(CONFIG.read_text())
    candidate = json.loads(CANDIDATE.read_text())
    assert candidate["animal_toxin"].pop(OVERRIDE) == 0.50 and candidate == reference
    assert reference["toxin"]["hp_damage_per_excess_unit"] == 0.01
    clearance = reference["toxin"]["clearance_fraction_per_tick"]
    threshold = reference["animal_toxin"]["carnivore_damage_threshold"]
    assert threshold == 0.05
    OUT.mkdir()
    manifest = {"binary_sha256": sha256(binary), "conditions": []}
    used = 0
    for seed in SEEDS:
        reference_decisions = None
        for tag, config in (("001", CONFIG), ("050", CANDIDATE)):
            row = {"seed": seed, "coefficient": float(tag) / 100, "config_sha256": sha256(config)}
            try:
                run = run_condition(binary, config, seed)
                if tag == "001":
                    with gzip.open(BASE / f"{seed}-005-trace.json.gz", "rt") as stream:
                        saved = json.load(stream)
                    assert run["rows"] == saved["rows"] and run["first_zero"] == saved["first_zero"]
                    assert [e for e in run["trace"]
                            if e["event"] not in ("opportunity", "clearance", "removal")] == saved["trace"]
                result, decisions = analyze(run, clearance, threshold)
                if tag == "001":
                    reference_decisions = decisions
                elif reference_decisions is not None:
                    result["first_observed_decision_difference"] = next(
                        ([old, new] for old, new in zip(reference_decisions, decisions)
                         if old != new), None
                    )
                    result["decision_sequence_lengths"] = [len(reference_decisions), len(decisions)]
                data = gzip.compress(json.dumps(result).encode(), mtime=0)
                assert used + len(data) <= LIMIT - 16384, "persistent output limit"
                path = OUT / f"{seed}-{tag}.json.gz"
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
