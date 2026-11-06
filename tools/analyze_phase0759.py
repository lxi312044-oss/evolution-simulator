"""Validate paired interventions before reporting whole-system (not individual) effects."""
import argparse
import csv
import json
import math
from pathlib import Path

SEEDS = (12345, 12346)
ENVIRONMENTS = {"A80": 0.8, "B80": 0.2, "equal": 0.5}
COUNTS = ("herbivores", "toxic_plants", "births_in_window", "deaths_in_window",
          "cumulative_herbivore_births", "cumulative_herbivore_deaths")
SHARED = ("herbivores", "births_in_window", "deaths_in_window",
          "cumulative_herbivore_births", "cumulative_herbivore_deaths")
METRICS = ("mean_resistance", "resistance_variance", "mean_load",
           "high_resistance_fraction", "mean_parent_resistance", "selection_differential",
           "mean_resistance_of_deaths", "mean_resistance_of_newborns")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_run(root, run, seed, fraction, protection, ticks, interval):
    config = json.loads((root / f"{run}.json").read_text(encoding="utf-8-sig"))
    toxin = config["toxin"]
    require(config["seed"] == seed, f"{run}: config seed mismatch")
    expected = dict(enabled=True, resistance_enabled=True, herbivore_multi_toxin_enabled=True,
                    plant_toxin_type_variation_enabled=True, plant_toxin_type_count=2,
                    initial_type0_fraction=fraction, additional_type_resistance_upkeep_multiplier=1.0,
                    multi_type_resistance_upkeep_multiplier=0.5,
                    resistance_effectiveness=0.8 if protection == "on" else 0.0)
    for key, value in expected.items():
        require(toxin.get(key) == value, f"{run}: unexpected toxin.{key}")
    with (root / f"{run}.types.csv").open(encoding="utf-8-sig", newline="") as handle:
        reader = csv.DictReader(handle)
        fields = reader.fieldnames or []
        require(len(fields) == len(set(fields)), f"{run}: duplicate columns")
        require(set(("seed", "tick", "toxin_type") + COUNTS + METRICS) <= set(fields),
                f"{run}: missing columns")
        rows = list(reader)
    by_type = {0: [], 1: []}
    for row in rows:
        require(None not in row and all(v is not None for v in row.values()), f"{run}: malformed CSV")
        for key in ("seed", "tick", "toxin_type") + COUNTS:
            row[key] = int(row[key])
            require(row[key] >= 0, f"{run}: negative {key}")
        for key in METRICS:
            row[key] = float(row[key])
            require(math.isfinite(row[key]), f"{run}: nonfinite {key}")
            if key != "selection_differential":
                require(row[key] >= 0, f"{run}: negative {key}")
        for key in ("mean_resistance", "high_resistance_fraction", "mean_parent_resistance",
                    "mean_resistance_of_deaths", "mean_resistance_of_newborns"):
            require(row[key] <= 1, f"{run}: out-of-range {key}")
        require(row["seed"] == seed, f"{run}: CSV seed mismatch")
        require(row["toxin_type"] in by_type, f"{run}: unexpected toxin type")
        by_type[row["toxin_type"]].append(row)
    expected_ticks = list(range(0, ticks + 1, interval))
    if expected_ticks[-1] != ticks:
        expected_ticks.append(ticks)
    for ty, samples in by_type.items():
        require([r["tick"] for r in samples] == expected_ticks,
                f"{run}: type {ty} tick coverage/order/duplicates mismatch")
        initial_n = config["initial_population"]["herbivores"]
        previous_births = previous_deaths = 0
        for row in samples:
            births, deaths = row["cumulative_herbivore_births"], row["cumulative_herbivore_deaths"]
            require(births >= previous_births and deaths >= previous_deaths,
                    f"{run}: decreasing cumulative counts")
            require(row["herbivores"] == initial_n + births - deaths,
                    f"{run}: population balance mismatch")
            require(row["births_in_window"] == births - previous_births and
                    row["deaths_in_window"] == deaths - previous_deaths,
                    f"{run}: window event count mismatch")
            previous_births, previous_deaths = births, deaths
    for a, b in zip(by_type[0], by_type[1]):
        require(all(a[key] == b[key] for key in SHARED), f"{run}: type population/count mismatch")
    # Only the defense setting may differ within a paired intervention.
    del toxin["resistance_effectiveness"]
    return config, by_type


def analyze(root, ticks=10000, interval=100):
    root = Path(root)
    require(ticks > 0 and interval > 0, "ticks and interval must be positive")
    expected = {f"seed{s}_{e}_{p}" for s in SEEDS for e in ENVIRONMENTS for p in ("on", "off")}
    actual = {p.name.removesuffix(".types.csv") for p in root.glob("*.types.csv")}
    require(actual == expected, f"Run matrix mismatch: missing={sorted(expected-actual)}, extra={sorted(actual-expected)}")
    summaries, pairs = [], []
    for seed in SEEDS:
        for environment, fraction in ENVIRONMENTS.items():
            arms = {}
            configs = {}
            initial = {}
            for protection in ("on", "off"):
                run = f"seed{seed}_{environment}_{protection}"
                configs[protection], samples = read_run(root, run, seed, fraction, protection, ticks, interval)
                initial[protection] = samples
                arms[protection] = {}
                for ty, data in samples.items():
                    first, last = data[0], data[-1]
                    n = last["herbivores"]
                    extinction = next((i for i, row in enumerate(data) if not row["herbivores"]), None)
                    row = dict(run=run, seed=f"seed{seed}", environment=environment,
                               protection=protection, toxin_type=ty, final_tick=ticks,
                               initial_herbivores=first["herbivores"], final_herbivores=n,
                               initial_resistance=first["mean_resistance"] if first["herbivores"] else "",
                               final_resistance=last["mean_resistance"] if n else "",
                               resistance_delta=last["mean_resistance"]-first["mean_resistance"] if n and first["herbivores"] else "",
                               initial_toxic_plants=first["toxic_plants"], final_toxic_plants=last["toxic_plants"],
                               total_births=last["cumulative_herbivore_births"], total_deaths=last["cumulative_herbivore_deaths"],
                               last_alive_sample_tick=data[extinction-1]["tick"] if extinction else "",
                               first_extinct_sample_tick=data[extinction]["tick"] if extinction is not None else "")
                    summaries.append(row)
                    arms[protection][ty] = row
            require(configs["on"] == configs["off"], f"seed{seed}_{environment}: paired configs differ beyond defense")
            for ty in (0, 1):
                require(initial["on"][ty][0] == initial["off"][ty][0],
                        f"seed{seed}_{environment}: paired initial states differ")
                on, off = arms["on"][ty], arms["off"][ty]
                pair = dict(seed=f"seed{seed}", environment=environment, toxin_type=ty,
                            on_extinct=on["final_herbivores"] == 0, off_extinct=off["final_herbivores"] == 0)
                for key in ("final_herbivores", "total_births", "total_deaths", "resistance_delta"):
                    pair[f"{key}_on_minus_off"] = on[key]-off[key] if on[key] != "" and off[key] != "" else ""
                pairs.append(pair)
    return summaries, pairs


def write_csv(path, rows):
    temporary = path.with_suffix(path.suffix + ".tmp")
    with temporary.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    temporary.replace(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=Path("results/phase0759"))
    parser.add_argument("--ticks", type=int, default=10000)
    parser.add_argument("--interval", type=int, default=100)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    out = args.out or args.root
    out.mkdir(parents=True, exist_ok=True)
    try:
        rows, pairs = analyze(args.root, args.ticks, args.interval)
    except (ValueError, OSError, KeyError) as error:
        (out / "quality.json").write_text(json.dumps({"passed": False, "error": str(error)}, indent=2), encoding="utf-8")
        parser.exit(2, f"FAIL: {error}\n")
    write_csv(out / "paired_summary.csv", rows)
    write_csv(out / "paired_effects.csv", pairs)
    (out / "quality.json").write_text(json.dumps(dict(passed=True, runs=12, type_rows=len(rows), pairs=len(pairs),
        ticks=args.ticks, interval=args.interval, scope="types CSV and paired configuration only; not individual event or provenance validation"), indent=2), encoding="utf-8")
    print(f"PASS: 12 runs, {len(rows)} type rows, {len(pairs)} paired type effects")
    print("WARNING: two seeds and ecosystem divergence do not establish genetic evolution or individual causality.")
    print("WARNING: extinction is interval-censored; missing resistance is not zero. Consult quality.json before using outputs.")


if __name__ == "__main__":
    main()
