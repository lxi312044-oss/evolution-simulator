"""Inventory archived focal resistance pairs and compare direct reproduction timing."""
import argparse
import json
import re
from collections import Counter
from pathlib import Path

from analyze_reproduction_health import load, read_rows
from run_confirmation import require
from run_longterm_resistance import f32
from run_metadata import sha256
from run_resistance_pilot import digest_json

ROOT = Path("results")
CATEGORIES = ("防御利益はあるが他条件で止まる", "防御利益が出生増加につながる",
              "不利益が上回る可能性", "証拠不足")


def terminal(row):
    return dict(tick=row["tick"], hp=row["hp"], dose=row.get("dose", row.get("dose_a", 0) + row.get("dose_b", 0)),
                damage=row["damage"], upkeep=row.get("upkeep", row.get("cost")),
                alive=row.get("alive", row.get("kind") == "survivor"), births=row["births"])


def point(row):
    return dict(hp=row["hp"], dose=row["dose"], damage=row["damage"], upkeep=row["upkeep"],
                reserve_e=row.get("reserve_e"))


def wasm_arm(directory, name, input_name, digest):
    require((directory / input_name).is_file(), "Missing saved input")
    path = directory / f"{name}.jsonl"
    if not path.exists():
        path = directory / f"{name}.jsonl.gz"
    require(sha256(path) == digest, f"Changed log: {path}")
    receipt = initial = end = None
    births = []
    samples = {}
    feeding = 0.0
    has_probe = False
    for row in read_rows(path):
        kind = row["kind"]
        if kind in ("probe_config", "intervention_audit"):
            receipt = row["receipt"]
        elif kind == "initial":
            initial = row
        elif kind == "probe":
            has_probe = True
            feeding += sum(e["delta_e"] for e in row["energy"] if e["phase"] == "feeding")
            if row["focal"] is not None:
                samples[row["tick"]] = point(row["focal"])
        elif kind == "birth" and row["parent"] == receipt["id"]:
            births.append(row["tick"])
        elif kind in ("removal", "survivor") and row["id"] == receipt["id"]:
            end = terminal(row)
    require(end["births"] == len(births), f"Birth mismatch: {path}")
    config = initial["config"]
    adult_hp = float(re.search(r"adult_max_hp: ([\d.eE+-]+)", receipt["before"])[1])
    threshold = f32(f32(adult_hp) * f32(config["herbivores"]["reproduction_min_hp_fraction"]))
    state = initial["initial_state"].copy()
    require(state["animals"].count(receipt["after"]) == 1, "Cannot identify focal initial state")
    state["animals"] = state["animals"].replace(receipt["after"], receipt["before"], 1)
    return dict(config=config, background=digest_json(state), horizon=initial["ticks"],
                threshold=threshold, end=end, samples=samples, birth_ticks=births,
                energy_threshold=f32(adult_hp) * config["herbivores"]["max_energy_per_hp"]
                * config["herbivores"]["reproduction_min_energy_fraction"],
                direct_births=len(births), feeding_e=feeding if has_probe else None,
                evidence=str(path), log_sha256=digest)


def native_arm(path, digest):
    require(sha256(path) == digest, f"Changed log: {path}")
    samples = {}
    births = []
    for row in read_rows(path):
        kind = row["kind"]
        if kind == "initial":
            initial = row
        elif kind == "snapshot" and row["focal"] and row["focal"]["alive"]:
            samples[row["tick"]] = point(terminal(row["focal"]))
        elif kind == "birth" and row["parent"] == initial["focal_id"]:
            births.append(row["tick"])
        elif kind == "complete":
            end = terminal(row["legacy_result"]["outcome"])
    require(end["births"] == len(births), f"Birth mismatch: {path}")
    return dict(config=initial["effective_config"], background=initial["initial_background_sha256"],
                horizon=initial["ticks"], threshold=None, end=end, samples=samples,
                birth_ticks=births, direct_births=len(births), feeding_e=None,
                evidence=str(path), log_sha256=digest)


def native_summary(row, path):
    protection = "focal_births" in row
    return dict(config=row["effective_config"],
                background=row["initial_background"] if protection else row["initial_background_sha256"],
                horizon=row["ticks"], threshold=None, end=None if protection else terminal(row["outcome"]),
                samples={}, birth_ticks=None, direct_births=row["focal_births"] if protection else row["outcome"]["births"],
                feeding_e=None, evidence=str(path), log_sha256=sha256(path))


def pair(low, high):
    require(low["background"] == high["background"], "Focal initial background differs")
    require(low["config"] == high["config"], "Pair configuration differs")
    require(low["horizon"] == high["horizon"], "Different horizons within pair")
    common = sorted(low["samples"].keys() & high["samples"].keys())
    shared_tick = common[-1] if common else None
    comparisons = Counter()
    high_hp_gap_with_low_e = 0
    for tick in common:
        if low["threshold"] is None or high["threshold"] is None:
            break
        l = low["samples"][tick]["hp"] >= low["threshold"]
        h = high["samples"][tick]["hp"] >= high["threshold"]
        comparisons["both_above" if l and h else "both_below" if not l and not h else
                    "high_above_low_below" if h else "low_above_high_below"] += 1
        high_hp_gap_with_low_e += (h and not l and high["samples"][tick]["reserve_e"] <
                                   high["energy_threshold"] and tick not in high["birth_ticks"])
    birth_delta = high["direct_births"] - low["direct_births"]
    birth_times = sorted(set(low["birth_ticks"] or []) | set(high["birth_ticks"] or []))
    discordant = sorted(set(low["birth_ticks"] or []) ^ set(high["birth_ticks"] or []))
    first_birth_difference = discordant[0] if discordant else None
    before_difference = max((t for t in common if first_birth_difference is not None
                             and t < first_birth_difference), default=None)
    same_tick = {arm: data["samples"].get(shared_tick) for arm, data in (("low", low), ("high", high))}
    common_diffs = {k: same_tick["high"][k] - same_tick["low"][k] for k in ("dose", "damage", "upkeep")} if common else None
    # This is a candidate for cost/behavior follow-up, not a causal classification.
    cost_evidence = (before_difference is not None and
                     high["samples"][before_difference]["upkeep"] >
                     low["samples"][before_difference]["upkeep"])
    category = CATEGORIES[2] if birth_delta < 0 and cost_evidence else CATEGORIES[3]
    out = dict(category=category, direct_birth_delta=birth_delta, common_tick=shared_tick,
               at_common_tick=same_tick, common_tick_high_minus_low=common_diffs,
               common_tick_count=len(common),
               first_birth_difference=first_birth_difference,
               common_tick_before_birth_difference=before_difference,
               before_birth_difference={arm: data["samples"].get(before_difference)
                                        for arm, data in (("low", low), ("high", high))},
               hp_states_at_shared_samples=dict(comparisons),
               high_hp_advantage_and_post_step_e_unmet=high_hp_gap_with_low_e,
               missing=["paired pre-birth gate state (especially satiety and E)",
                        "matched toxin burden and clearance history"])
    config = low["config"]["toxin"]
    if not config["herbivore_multi_toxin_enabled"] and config["enabled"] and config["resistance_enabled"]:
        out["fixed_raw_damage_multipliers"] = {
            str(r): 1 - min(1, f32(r) * config["resistance_effectiveness"])
            for r in (0.2, 0.8)
        }
        matched = None
        for tick in common:
            if tick != (0 if matched is None else matched + 1):
                break
            if low["samples"][tick]["dose"] != high["samples"][tick]["dose"]:
                break
            matched = tick
        if matched is not None:
            out["equal_tickwise_dose_prefix"] = dict(
                through_tick=matched, dose=low["samples"][matched]["dose"],
                low_damage=low["samples"][matched]["damage"],
                high_damage=high["samples"][matched]["damage"])
    for arm, data in (("low", low), ("high", high)):
        threshold = data["threshold"]
        first_below = next((tick for tick, row in sorted(data["samples"].items())
                            if threshold is not None and row["hp"] < threshold), None)
        last_above = max((tick for tick, row in data["samples"].items()
                          if first_below is not None and tick < first_below and row["hp"] >= threshold), default=None)
        out[arm] = {k: v for k, v in data.items() if k not in ("config", "background", "samples")}
        out[arm].update(first_sample_below=first_below, previous_sample_above=last_above,
                        post_step_at_either_birth={t: data["samples"].get(t) for t in birth_times})
        if first_below is not None and last_above is not None:
            before, after = [data["samples"][t] for t in (last_above, first_below)]
            out[arm]["crossing_bracket"] = dict(
                from_tick=last_above, to_tick=first_below, hp_loss=before["hp"]-after["hp"],
                recorded_toxin_damage=after["damage"]-before["damage"])
    return out


def pilot_gate_details(low_path, high_path):
    traces = ROOT / "focal-diagnosis-20261107/runs"
    paths = [traces / p.name.replace(".result.json", ".trace.json") for p in (low_path, high_path)]
    if not all(p.exists() for p in paths):
        return None
    arms = []
    for path, reference in zip(paths, (low_path, high_path)):
        data, pilot = load(path), load(reference)
        require(all(data["result"][k] == pilot[k] for k in ("outcome", "initial_background_sha256")),
                "Diagnostic trace differs from the pilot")
        gates = {r["tick"]: r for r in data["trace"] if r["phase"] == "reproduction_gates"}
        arms.append(gates)
    low, high = arms
    counts = Counter(
        f"low_{low[t]['hp_ok']}_high_{high[t]['hp_ok']}" for t in sorted(low.keys() & high.keys())
    )
    return dict(shared_hp_gate_states=dict(counts),
                energy_and_satiety_pass=[sum(r["energy_ok"] and r["transfer_e_ok"] and
                                             r["satiety_ok"] for r in arm.values()) for arm in arms],
                evaluated_ticks=[len(arm) for arm in arms],
                source_sha256={str(path): sha256(path) for path in paths})


def inventory():
    groups = {}
    directory = ROOT / "focal-intervention-longterm-v3-20261108"
    execution = load(directory / "execution.json")
    long = {}
    for seed in range(12345, 12353):
        for toxic in (0, 1):
            arms = []
            for resistance in (0.2, 0.8):
                name = f"seed{seed}_toxin{toxic}_r{resistance}"
                arms.append(wasm_arm(directory, name, name + ".input.json",
                                     execution["output_sha256"][name + ".jsonl.gz"]))
            long[f"{seed}_{toxic}"] = pair(*arms)
    groups["wasm_long"] = long

    for label, dirname, seeds, fractions, suffix in (
        ("wasm_fraction_pilot", "focal-toxic-fraction-20261109", (12350, 12351), (0.1, 0.9), "_paid"),
        ("wasm_two_cycles", "focal-cycle-comparison-20261109", range(12353, 12361), (0.1, 0.9), ""),
        ("wasm_two_cycles_replication", "focal-cycle-rich-replication-20261109", range(12361, 12369), (0.9,), ""),
    ):
        directory = ROOT / dirname
        saved = load(directory / "assessment.json")
        saved = saved.get("runs", saved)
        groups[label] = {}
        for seed in seeds:
            for fraction in fractions:
                arms = []
                for resistance in (0.2, 0.8):
                    name = f"{seed}_{fraction}_{resistance}{suffix}"
                    arm = wasm_arm(directory, name, name + ".input.json", saved[name]["log_sha256"])
                    require(arm["direct_births"] == saved[name]["terminal"]["births"], "Saved focal births differ")
                    arms.append(arm)
                groups[label][f"{seed}_{fraction}"] = pair(*arms)

    directory = ROOT / "confirmation-study-20261107" / "runs"
    groups["native_confirmation"] = {}
    for low_result in sorted(directory.glob("*_low_*.result.json")):
        arms = []
        for result in (low_result, low_result.with_name(low_result.name.replace("_low_", "_high_"))):
            summary = load(result)
            arm = native_arm(result.with_name(result.name.replace(".result.json", ".jsonl.gz")), summary["log_sha256"])
            require(arm["direct_births"] == summary["endpoints"][str(summary["ticks"])]["direct_births"],
                    "Native focal birth mismatch")
            arms.append(arm)
        groups["native_confirmation"][low_result.name.removesuffix(".result.json").replace("_low_", "_")] = pair(*arms)

    for label, directory, pattern in (
        ("native_pilot", ROOT / "resistance-pilot-20261107/pilot", "*_low_*.result.json"),
        ("native_plant_protection", ROOT / "plant-protection-comparison-20261107/runs", "*_low_*.json"),
    ):
        groups[label] = {}
        for low_path in sorted(directory.glob(pattern)):
            high_path = low_path.with_name(low_path.name.replace("_low_", "_high_"))
            arms = [native_summary(load(path), path) for path in (low_path, high_path)]
            comparison = pair(*arms)
            if label == "native_pilot":
                comparison["diagnostic_gates"] = pilot_gate_details(low_path, high_path)
            groups[label][low_path.name.replace("_low_", "_")] = comparison
    return groups


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    groups = inventory()
    with args.output.open("x", encoding="utf-8") as stream:
        json.dump(groups, stream, ensure_ascii=False, indent=2)
    for name, pairs in groups.items():
        print(name, len(pairs), Counter((p["direct_birth_delta"] > 0) - (p["direct_birth_delta"] < 0)
                                        for p in pairs.values()), flush=True)
