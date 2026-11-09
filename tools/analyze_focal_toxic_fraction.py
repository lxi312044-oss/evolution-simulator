"""Audit the paired 7000-tick toxic-founder-fraction experiment."""
import argparse
import json
import re
from pathlib import Path

from run_confirmation import require
from run_metadata import sha256
from run_wasm_lineage import FitnessAudit


def summarize(path, config):
    audit = FitnessAudit(config, 7000)
    probes = []
    receipt = None
    terminal = None
    last_feeding = None
    feeding = 0.0
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line)
            kind = row["kind"]
            if kind == "probe_config":
                receipt = row["receipt"]
            elif kind == "probe":
                probes.append(row)
                for event in row["energy"]:
                    if event["phase"] == "feeding" and event["delta_e"] > 0:
                        last_feeding = row["tick"]
                        feeding += event["delta_e"]
            else:
                audit.accept(row)
                if kind in ("removal", "survivor") and row["id"] == receipt["id"]:
                    terminal = row
    audit.finish()
    require([r["tick"] for r in probes] == list(range(7001)), "Incomplete probes")
    require(terminal is not None, "Missing focal terminal record")
    state = audit.initial["initial_state"].copy()
    require(state["animals"].count(receipt["after"]) == 1, "Missing focal state")
    state["animals"] = state["animals"].replace(receipt["after"], receipt["before"], 1)
    state["plants"] = re.sub(r"produces_toxin: (true|false)", "produces_toxin: masked", state["plants"])
    first_zero = next((r["tick"] for r in probes
                       if r["focal"] and r["focal"]["reserve_e"] <= 1e-9), None)
    result = dict(terminal=terminal, feeding_e=feeding, last_feeding=last_feeding,
                  first_zero=first_zero, initial_plants=probes[0]["plants"],
                  initial_toxic_plants=probes[0]["toxic_plants"],
                  plant_counts=[{k: row[k] for k in ("tick", "plants", "toxic_plants")}
                                for row in probes[::1000]],
                  birth_ticks=audit.life[tuple(receipt["id"])]["children"],
                  log_sha256=sha256(path), max_n_error=audit.complete["max_n_error"],
                  max_e_error=audit.complete["max_e_error"])
    return result, state


def main(directory):
    results = {}
    for seed in (12350, 12351):
        baseline_state = None
        for fraction in (0.1, 0.9):
            for resistance, mode in ((0.2, "paid"), (0.8, "paid"), (0.8, "exempt")):
                if fraction == 0.9 and mode == "exempt":
                    continue
                name = f"{seed}_{fraction}_{resistance}_{mode}"
                config = json.loads((directory / f"{name}.input.json").read_text(encoding="utf-8"))
                result, state = summarize(directory / f"{name}.jsonl", config)
                if baseline_state is None:
                    baseline_state = state
                require(state == baseline_state, f"Initial state mismatch: {name}")
                require(result["initial_plants"] == 1200, "Founder plant count mismatch")
                require(result["terminal"]["resistance"] == resistance or
                        abs(result["terminal"]["resistance"] - resistance) < 1e-7, "Resistance mismatch")
                require((result["terminal"]["upkeep"] == 0) == (mode == "exempt"), "Upkeep mismatch")
                results[name] = result
    with (directory / "assessment.json").open("x", encoding="utf-8") as stream:
        json.dump(results, stream, indent=2)
    for name, result in results.items():
        end = result["terminal"]
        print(name, result["initial_toxic_plants"], end["tick"], end["kind"],
              end["births"], round(end["dose"], 3), round(end["damage"], 3),
              round(end["upkeep"], 3), result["first_zero"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    main(parser.parse_args().directory)
