"""Detect paired observation windows from untreated focal-cycle baselines."""
import argparse
import json
from pathlib import Path
from statistics import mean

from run_metadata import sha256


def detect(rows, plan, window):
    smoothed = [
        (row["tick"], mean(r["living"] for r in rows[max(0, i-window+1):i+1]),
         mean(r["plants"] for r in rows[max(0, i-window+1):i+1]))
        for i, row in enumerate(rows)
    ]
    floor = peak = trough = smoothed[0]
    plant_min = peak[2]
    stage = "rise"
    cycles = []
    for point in smoothed[1:]:
        tick, living, plants = point
        if stage == "rise":
            if living < floor[1]:
                floor = point
            if living >= max(floor[1] + plan["peak_min_gain"],
                             floor[1] * plan["peak_min_multiple"]):
                peak, plant_min, stage = point, plants, "decline"
        elif stage == "decline":
            if living > peak[1]:
                peak, plant_min = point, plants
            plant_min = min(plant_min, plants)
            if living <= peak[1] * plan["trough_max_peak_fraction"]:
                trough, stage = point, "rebound"
        else:
            plant_min = min(plant_min, plants)
            if living < trough[1]:
                trough = point
            if (living >= max(trough[1] + plan["rebound_min_gain"],
                              trough[1] * plan["rebound_min_multiple"])
                    and plants >= plant_min * plan["plant_recovery_multiple"]):
                cycles.append(dict(peak_tick=peak[0], peak_living=peak[1],
                                   trough_tick=trough[0], trough_living=trough[1],
                                   end_tick=tick, end_living=living,
                                   minimum_plants=plant_min, end_plants=plants))
                if len(cycles) == 2:
                    break
                floor, stage = trough, "rise"
    return cycles


def main(directory):
    plan = json.loads((directory / "plan.json").read_text(encoding="utf-8"))
    outcomes = {}
    for seed, fraction in plan["cases"]:
        name = f"{seed}_{fraction}"
        path = directory / f"{name}.jsonl"
        with path.open(encoding="utf-8") as stream:
            rows = [json.loads(line) for line in stream]
        if [r["tick"] for r in rows] != list(range(0, plan["cap"]+1, plan["interval"])):
            raise ValueError(f"Incomplete baseline: {name}")
        if any(r["living"] < 0 or r["plants"] < 0 for r in rows):
            raise ValueError(f"Negative population: {name}")
        config = json.loads((directory / f"{name}.input.json").read_text(encoding="utf-8"))
        if (config["seed"], config["toxin"]["initial_toxic_plant_fraction"]) != (seed, fraction):
            raise ValueError(f"Incorrect configuration: {name}")
        cycles = detect(rows, plan, plan["window"])
        sensitivity = {width: [c["end_tick"] for c in detect(rows, plan, width)]
                       for width in (3, 7)}
        outcomes[name] = dict(cycles=cycles, complete=len(cycles)==2,
                              sensitivity=sensitivity, log_sha256=sha256(path))
        print(name, [(c["peak_tick"], c["trough_tick"], c["end_tick"]) for c in cycles],
              sensitivity)
    with (directory / "assessment.json").open("x", encoding="utf-8") as stream:
        json.dump(outcomes, stream, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    main(parser.parse_args().directory)
