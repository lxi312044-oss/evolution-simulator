"""Run the fixed Stage 4 observation and audit the previously saved trajectory."""
import gzip
import json
import math
import subprocess
from collections import Counter, defaultdict
from pathlib import Path

BASE = Path("results/stage4-carnivore-threshold")
CONFIG = Path("config/stage4-carnivore-threshold-005.json")
OUT = Path("results/stage4-predator-opportunities")
SEEDS = (12345, 12346, 12347)


def close(a, b):
    assert math.isclose(a, b, rel_tol=0., abs_tol=1e-9), (a, b)


def summarize(run, clearance, threshold):
    counts = Counter()
    individuals = defaultdict(set)
    first_transfer, first_memory, histories = {}, {}, {}
    load = defaultdict(lambda: [0., 0.])
    totals = {key: [0., 0.] for key in ("transfer", "clearance", "removal")}
    windows = defaultdict(Counter)

    def count(stage, identity, n=1):
        if n:
            counts[stage] += n
            individuals[stage].add(identity)
            windows[(tick - 1) // 500][stage] += n

    for event in run["trace"]:
        tick, identity = event["tick"], tuple(event["id"])
        kind = event["event"]
        if kind in ("transfer", "damage", "clearance"):
            key = identity, event["type"]
            stock = load[key]
        if kind == "transfer":
            close(sum(stock), event["load_before"])
            origin = [event["residual"], event["produced"]]
            for j, q in enumerate(origin):
                stock[j] += q
                totals["transfer"][j] += q
            close(sum(stock), event["load_after"])
            count("positive_type_transfer", identity)
            count("plant_type_transfer", identity, int(origin[0] > 0))
            first_transfer.setdefault((identity, event["rgb"], event["type"]), event)
        elif kind == "damage":
            count("damage_type_tick", identity)
            close(sum(stock), event["load_before_clearance"])
            plant, produced = (q * (1 - clearance) for q in stock)
            close(plant + produced, event["load_after_clearance"])
            count("damage_with_plant", identity, int(plant > 0))
            count("plant_alone_exceeds", identity, int(plant > threshold))
            count("plant_needed_to_exceed", identity, int(plant > 0 and produced <= threshold < plant + produced))
            before = dict(event["memory_before"])
            for color, value in event["memory_after"]:
                if value > before.get(color, 0.):
                    count("memory_color_update", identity)
                    episode = identity, color, event["type"]
                    if episode not in first_memory and episode in first_transfer:
                        first_memory[episode] = tick
                        histories[episode] = {"transfer":first_transfer[episode],
                            "damage":event,"damage_origin_after_clearance":[plant, produced]}
        elif kind == "clearance":
            close(sum(stock), event["before"])
            for j, q in enumerate(stock):
                stock[j] = q * (1 - clearance)
                totals["clearance"][j] += q - stock[j]
            close(sum(stock), event["after"])
        elif kind == "removal":
            for toxin, expected in enumerate(event["load"]):
                dead = load.pop((identity, toxin), [0.,0.])
                close(sum(dead), expected)
                for j, q in enumerate(dead):
                    totals["removal"][j] += q
        elif kind == "opportunity":
            count("predator_tick", identity)
            plant, produced, either = event["visible"]
            count("visible_toxic_tick", identity, int(either > 0))
            count("visible_plant_tick", identity, int(plant > 0))
            count("visible_produced_tick", identity, int(produced > 0))
            count("attack_tick", identity, int(event["attacked"]))
            count("positive_bite_tick", identity, int(event["eaten_n"] > 1e-10))
            for color, (visible, eligible, rejected, eligible_rejected, original) in event["same_colors"].items():
                color = int(color)
                count("same_color_tick", identity)
                count("same_color_prey", identity, visible)
                count("original_prey_visible", identity, original)
                count("different_prey_visible", identity, visible-original)
                count("attack_ready_color_tick", identity, int(eligible > 0))
                count("attack_ready_prey", identity, eligible)
                count("color_rejected_prey", identity, rejected)
                count("attack_ready_color_rejected", identity, eligible_rejected)
                count("not_attack_ready_color_rejected", identity, rejected-eligible_rejected)
                selected = (event["target"] or {}).get("color") == color
                count("same_color_selected", identity, int(selected))
                count("same_color_approached", identity, int(selected and event["distance_after"] < event["target"]["distance_before"]))
                count("same_color_attacked", identity, int(selected and event["attacked"]))
                count("same_color_fed", identity, int(selected and event["eaten_n"] > 1e-10))
                for episode, row in histories.items():
                    if episode[:2] != (identity,color) or first_memory[episode] >= tick:
                        continue
                    row.setdefault("first_visible", event)
                    if eligible:
                        row.setdefault("first_attack_ready", event)
                    if selected and event["attacked"]:
                        row.setdefault("first_actual_attack", event)
    remaining = [sum(q[j] for q in load.values()) for j in range(2)]
    for j in range(2):
        close(totals["transfer"][j], totals["clearance"][j]+totals["removal"][j]+remaining[j])
    final = run["rows"][-1]["animal_toxin"]
    for j, field in enumerate(("transferred_residual","transferred_produced")):
        close(totals["transfer"][j], sum(final[field]))
    if run["rows"][-1]["counts"][2] == 0:
        assert all(v == 0. for v in remaining)
    return {"counts": counts, "individuals": {k:len(v) for k,v in individuals.items()},
            "windows":dict(windows), "origins":dict(totals,remaining=remaining),
            "histories":list(histories.values()), "seconds":run["seconds"],
            "max_population":run["max_population"], "max_n_error":run["max_n_error"],
            "max_e_error":run["max_e_error"], "max_toxin_error":run["max_sampled_toxin_error"]}


def main():
    OUT.mkdir()
    cfg = json.loads(CONFIG.read_text())
    for seed in SEEDS:
        with gzip.open(BASE/f"{seed}-005-trace.json.gz","rt") as f:
            saved = json.load(f)
        completed = subprocess.run(
            ["target/release/examples/stage4_integrated_baseline.exe", str(CONFIG), str(seed), "opportunities"],
            capture_output=True, check=True, timeout=120,
        )
        run = json.loads(completed.stdout)
        assert run["rows"] == saved["rows"] and run["first_zero"] == saved["first_zero"]
        assert [e for e in run["trace"] if e["event"] not in ("opportunity","clearance","removal")] == saved["trace"]
        summary = summarize(run,cfg["toxin"]["clearance_fraction_per_tick"],cfg["animal_toxin"]["carnivore_damage_threshold"])
        data = gzip.compress(json.dumps(summary).encode(),mtime=0)
        assert len(data)+sum(p.stat().st_size for p in OUT.iterdir()) <= 10*2**20
        with (OUT/f"{seed}.json.gz").open("xb") as f:
            f.write(data)
        print(seed,summary["seconds"],summary["counts"],flush=True)


if __name__ == "__main__":
    main()
