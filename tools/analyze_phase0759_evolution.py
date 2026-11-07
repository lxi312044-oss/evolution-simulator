"""Phase 07.5.9: descriptive lineage/exposure assessment with strict event checks."""
import argparse
from collections import Counter, defaultdict
import csv
import hashlib
import json
import math
from pathlib import Path

from analyze_phase0759 import analyze, require, write_csv

ROOT = Path(__file__).resolve().parents[1]
TOL = 2e-8


def mean(values):
    values = list(values)
    return sum(values) / len(values) if values else ""


def load_csv(path, integers, floats, strings=()):
    with path.open(encoding="utf-8-sig", newline="") as handle:
        reader = csv.DictReader(handle)
        fields = reader.fieldnames or []
        require(len(fields) == len(set(fields)), f"{path.name}: duplicate columns")
        require(set(integers + floats + strings) <= set(fields), f"{path.name}: missing columns")
        rows = []
        for row in reader:
            require(None not in row and all(v is not None for v in row.values()), f"{path.name}: malformed row")
            for key in integers:
                row[key] = int(row[key])
                require(row[key] >= 0, f"{path.name}: negative {key}")
            for key in floats:
                row[key] = float(row[key])
                require(math.isfinite(row[key]), f"{path.name}: nonfinite {key}")
                if key != "delta":
                    require(row[key] >= 0, f"{path.name}: negative {key}")
                if "resistance" in key:
                    require(row[key] <= 1, f"{path.name}: resistance outside [0,1]")
            rows.append(row)
        return rows


def identity(row, prefix="id"):
    return (row[f"{prefix}_index"], row[f"{prefix}_generation"])


def check_pairs(rows, key, shared, context):
    groups = defaultdict(dict)
    for row in rows:
        group = groups[key(row)]
        ty = row["toxin_type"]
        require(ty in (0, 1) and ty not in group, f"{context}: duplicate/invalid type")
        group[ty] = row
    for pair in groups.values():
        require(set(pair) == {0, 1}, f"{context}: missing paired type")
        require(all(pair[0][f] == pair[1][f] for f in shared), f"{context}: type event mismatch")
    return groups


def audit_run(root, run, ticks, interval):
    """Return one row per individual and type; validate all three event CSVs."""
    config = json.loads((root / f"{run}.json").read_text(encoding="utf-8-sig"))
    seed = config["seed"]
    births = load_csv(root / f"{run}.births.csv",
        ("seed", "tick", "toxin_type", "parent_id", "parent_generation", "child_id", "child_generation"),
        ("parent_resistance", "child_resistance", "delta"), ("mutation_observed",))
    samples = load_csv(root / f"{run}.individuals.csv",
        ("seed", "tick", "toxin_type", "id_index", "id_generation", "age", "successful_births"),
        ("resistance", "lifetime_exposure", "load"), ("event",))
    outcomes = load_csv(root / f"{run}.outcomes.csv",
        ("seed", "tick", "toxin_type", "id_index", "id_generation", "age", "successful_births"),
        ("resistance", "load"), ("event",))
    for row in births + samples + outcomes:
        require(row["seed"] == seed and row["tick"] <= ticks, f"{run}: seed/tick mismatch")
    exact_terminal = bool(samples) and "observation_phase" in samples[0]
    if exact_terminal:
        for row in samples + outcomes:
            expected_phase = "pre_removal_current_tick" if row["event"] == "death" else "end_tick"
            require(row.get("observation_phase") == expected_phase, f"{run}: terminal phase mismatch")
            trigger = row.get("removal_trigger")
            require(trigger in (("hp_nonpositive", "age_limit") if row["event"] == "death" else ("none",)),
                    f"{run}: invalid removal trigger")
            row["hp"] = float(row["hp"])
            require(math.isfinite(row["hp"]), f"{run}: nonfinite hp")
            if row["event"] == "death":
                require((trigger == "hp_nonpositive") == (row["hp"] <= 0), f"{run}: trigger/hp mismatch")
        for row in samples:
            for key in ("toxin_damage_total", "prevented_damage_total", "resistance_cost_total"):
                row[key] = float(row[key])
                require(math.isfinite(row[key]), f"{run}: invalid physiology total")
            require(row["toxin_damage_total"] >= 0 and row["resistance_cost_total"] >= 0,
                    f"{run}: negative damage/cost")
            # Existing kernel subtracts f32-applied damage from f64 raw damage.
            # With defense OFF this signed rounding residual can be negative.
            # Preserve it; allow only the f32 relative rounding bound + CSV precision.
            rounding_bound = 2**-23 * row["toxin_damage_total"] + 1e-9
            require(row["prevented_damage_total"] >= -rounding_bound,
                    f"{run}: prevented damage exceeds rounding bound")
    events = check_pairs(births, lambda r: (r["child_id"], r["child_generation"]),
                         ("tick", "parent_id", "parent_generation"), run + " births")
    snapshots = check_pairs(samples, lambda r: (r["tick"], r["event"], identity(r)),
                            ("age", "successful_births"), run + " individuals")
    if exact_terminal:
        for pair in snapshots.values():
            require(all(pair[0][key] == pair[1][key] for key in
                        ("hp", "removal_trigger", "toxin_damage_total", "prevented_damage_total", "resistance_cost_total")),
                    f"{run}: shared physiology differs between types")
    deaths = check_pairs(outcomes, lambda r: (r["tick"], identity(r)),
                        ("age", "successful_births", "event"), run + " outcomes")
    nodes = {}
    for (tick, event, ident), pair in snapshots.items():
        require(event in ("alive", "death"), f"{run}: unknown event")
        if event == "alive":
            require(tick % interval == 0 or tick == ticks, f"{run}: invalid sample tick")
        else:
            require(tick > 0, f"{run}: death at initial tick")
        if tick == 0:
            require(event == "alive", f"{run}: invalid founder")
            nodes[ident] = dict(birth_tick=0, parent=None, depth=0, mutation={0: False, 1: False},
                               resistance={ty: pair[ty]["resistance"] for ty in (0, 1)})
    require(len(nodes) == config["initial_population"]["herbivores"], f"{run}: founder count")
    for child, pair in sorted(events.items(), key=lambda item: (item[1][0]["tick"], item[0])):
        first = pair[0]
        parent = (first["parent_id"], first["parent_generation"])
        require(child not in nodes and parent in nodes, f"{run}: duplicate child/missing parent")
        require(0 < first["tick"] <= ticks and nodes[parent]["birth_tick"] < first["tick"],
                f"{run}: parent birth ordering")
        for ty, row in pair.items():
            require(abs(row["parent_resistance"] - nodes[parent]["resistance"][ty]) <= TOL,
                    f"{run}: parent resistance mismatch")
            require(abs(row["child_resistance"] - row["parent_resistance"] - row["delta"]) <= TOL,
                    f"{run}: mutation delta mismatch")
            require(row["mutation_observed"] in ("true", "false"), f"{run}: invalid mutation flag")
            if row["mutation_observed"] == "false":
                require(abs(row["delta"]) <= TOL, f"{run}: false mutation has delta")
        nodes[child] = dict(birth_tick=first["tick"], parent=parent,
            depth=nodes[parent]["depth"] + 1,
            mutation={ty: pair[ty]["mutation_observed"] == "true" for ty in (0, 1)},
            resistance={ty: pair[ty]["child_resistance"] for ty in (0, 1)})
    histories = defaultdict(list)
    alive = Counter()
    for (tick, event, ident), pair in snapshots.items():
        require(ident in nodes, f"{run}: sample without founder/birth")
        require(tick >= nodes[ident]["birth_tick"], f"{run}: sample before birth")
        histories[ident].append((tick, event, pair))
        if event == "alive":
            alive[tick] += 1
        if event == "death":
            require((tick, ident) in deaths, f"{run}: death outcome missing")
            for ty in (0, 1):
                out = deaths[(tick, ident)][ty]
                require(out["event"] == "death", f"{run}: invalid outcome")
                require(all(out[k] == pair[ty][k] for k in ("age", "successful_births", "resistance", "load")),
                        f"{run}: outcome/snapshot mismatch")
                if exact_terminal:
                    require(out["hp"] == pair[ty]["hp"] and out["removal_trigger"] == pair[ty]["removal_trigger"],
                            f"{run}: outcome terminal mismatch")
    sample_deaths = {(t, ident) for t, event, ident in snapshots if event == "death"}
    require(set(deaths) == sample_deaths, f"{run}: unexpected death outcome")
    with (root / f"{run}.types.csv").open(newline="", encoding="utf-8-sig") as handle:
        type_rows = [r for r in csv.DictReader(handle) if r["toxin_type"] == "0"]
    for row in type_rows:
        tick = int(row["tick"])
        require(alive[tick] == int(row["herbivores"]), f"{run}: alive coverage mismatch")
        require(sum(pair[0]["tick"] <= tick for pair in events.values()) ==
                int(row["cumulative_herbivore_births"]), f"{run}: birth event count mismatch")
        require(sum(t <= tick for t, _ in deaths) ==
                int(row["cumulative_herbivore_deaths"]), f"{run}: death event count mismatch")
    children = Counter(node["parent"] for node in nodes.values() if node["parent"] is not None)
    for node in nodes.values():
        if node["parent"] is not None:
            parent_observations = histories[node["parent"]]
            require(bool(parent_observations) and
                    node["birth_tick"] <= max(t for t, _, _ in parent_observations),
                    f"{run}: child born after parent terminal")
    result = []
    for ident, node in nodes.items():
        observations = sorted(histories[ident], key=lambda item: item[0])
        require(bool(observations), f"{run}: unobserved individual")
        terminal = [(t, e, p) for t, e, p in observations if e == "death" or t == ticks]
        require(len(terminal) == 1 and terminal[0] == observations[-1], f"{run}: missing/duplicate terminal")
        terminal_tick, event, pair = terminal[0]
        for ty in (0, 1):
            previous_dose = previous_births = previous_age = -1
            for _, _, sample_pair in observations:
                sample = sample_pair[ty]
                require(abs(sample["resistance"] - node["resistance"][ty]) <= TOL,
                        f"{run}: nonheritable resistance change")
                require(sample["lifetime_exposure"] >= previous_dose and
                        sample["successful_births"] >= previous_births and sample["age"] >= previous_age,
                        f"{run}: decreasing individual counters")
                previous_dose, previous_births, previous_age = sample["lifetime_exposure"], sample["successful_births"], sample["age"]
            last = pair[ty]
            require(last["successful_births"] <= children[ident], f"{run}: unlogged children")
            children_before_death = sum(n["parent"] == ident and n["birth_tick"] < terminal_tick
                                        for n in nodes.values())
            if event == "death":
                require(last["successful_births"] == (children[ident] if exact_terminal else children_before_death),
                        f"{run}: missing births outside death tick")
            if event != "death":
                require(last["successful_births"] == children[ident], f"{run}: survivor birth mismatch")
            result.append(dict(run=run, seed=seed, environment=run.split("_")[1], protection=run.split("_")[2],
                toxin_type=ty, id_index=ident[0], id_generation=ident[1], generation_depth=node["depth"],
                founder=node["parent"] is None, birth_tick="" if node["parent"] is None else node["birth_tick"],
                birth_cohort="founder" if node["parent"] is None else str(node["birth_tick"] // 1000 * 1000),
                resistance_at_birth_or_initial=node["resistance"][ty],
                parent_resistance="" if node["parent"] is None else nodes[node["parent"]]["resistance"][ty],
                transmission_delta="" if node["parent"] is None else node["resistance"][ty]-nodes[node["parent"]]["resistance"][ty],
                mutation_observed=node["mutation"][ty],
                terminal_tick=terminal_tick, outcome="death" if event == "death" else "censored",
                exposure_terminal_lower_bound=last["lifetime_exposure"], logged_children=children[ident],
                terminal_successful_births_lower_bound=last["successful_births"],
                unobserved_death_tick_births=children[ident]-last["successful_births"],
                terminal_age_lower_bound=last["age"]))
            result[-1].update(
                current_terminal_schema=exact_terminal,
                terminal_exposure=last["lifetime_exposure"],
                terminal_age_observed=last["age"],
                terminal_successful_births_observed=last["successful_births"],
                terminal_observation_phase=last.get("observation_phase", "previous_tick" if event == "death" else "end_tick"),
                exposure_is_lower_bound=event == "death" and not exact_terminal,
                removal_trigger=last.get("removal_trigger", "unknown"),
                toxin_damage_all_types=last.get("toxin_damage_total", ""),
                prevented_damage_all_types=last.get("prevented_damage_total", ""),
                resistance_cost_all_types=last.get("resistance_cost_total", ""))
    return result


def summarize(rows, fields):
    groups = defaultdict(list)
    for row in rows:
        groups[tuple(row[k] for k in fields)].append(row)
    output = []
    for key, members in sorted(groups.items()):
        newborn = [r for r in members if not r["founder"]]
        output.append(dict(zip(fields, key)) | dict(n=len(members),
            deaths=sum(r["outcome"] == "death" for r in members),
            censored=sum(r["outcome"] == "censored" for r in members),
            mean_resistance=mean(r["resistance_at_birth_or_initial"] for r in members),
            mean_parent_resistance=mean(r["parent_resistance"] for r in newborn),
            mean_transmission_delta=mean(r["transmission_delta"] for r in newborn),
            mutation_fraction=mean(r["mutation_observed"] for r in newborn),
            exposed_n=sum(r["exposure_terminal_lower_bound"] > 0 for r in members),
            mean_exposure_lower_bound=mean(r["exposure_terminal_lower_bound"] for r in members),
            mean_logged_children=mean(r["logged_children"] for r in members),
            total_logged_children=sum(r["logged_children"] for r in members),
            missed_death_tick_births=sum(r["unobserved_death_tick_births"] for r in members)))
    return output


def compare_generations(generations):
    groups = defaultdict(dict)
    for row in generations:
        groups[(row["seed"], row["environment"], row["toxin_type"], row["generation_depth"])][row["protection"]] = row
    result = []
    for (seed, environment, ty, depth), arms in sorted(groups.items()):
        row = dict(seed=seed, environment=environment, toxin_type=ty, generation_depth=depth,
                   on_present="on" in arms, off_present="off" in arms)
        for metric in ("n", "mean_resistance", "mean_exposure_lower_bound", "mean_logged_children", "mean_transmission_delta"):
            a, b = arms.get("on", {}).get(metric, ""), arms.get("off", {}).get(metric, "")
            row[metric + "_on_minus_off"] = a-b if a != "" and b != "" else ""
        result.append(row)
    return result


def generation_components(generations):
    lookup = {(r["run"], r["toxin_type"], r["generation_depth"]): r for r in generations}
    result = []
    for row in generations:
        depth = row["generation_depth"]
        if depth == 0:
            continue
        previous = lookup[(row["run"], row["toxin_type"], depth-1)]
        # An arithmetic decomposition, not age/environment-adjusted natural selection.
        change = row["mean_resistance"] - previous["mean_resistance"]
        representation = row["mean_parent_resistance"] - previous["mean_resistance"]
        transmission = row["mean_transmission_delta"]
        residual = change - representation - transmission
        require(abs(residual) <= TOL, "Generation decomposition mismatch")
        result.append({k: row[k] for k in ("run", "seed", "environment", "protection", "toxin_type", "generation_depth", "n")} |
                      dict(previous_generation_n=previous["n"], mean_resistance_change=change,
                           parental_representation_component=representation,
                           transmission_component=transmission, arithmetic_residual=residual))
    return result


def fitness_associations(individuals):
    groups = defaultdict(list)
    for row in individuals:
        if row["founder"]:
            continue
        key = tuple(row[k] for k in ("run", "seed", "environment", "protection", "toxin_type", "birth_cohort"))
        groups[key + (row["exposure_terminal_lower_bound"] > 0,)].append(row)
    result = []
    for key, members in sorted(groups.items()):
        rs = [r["resistance_at_birth_or_initial"] for r in members]
        ws = [r["logged_children"] for r in members]
        mr, mw = mean(rs), mean(ws)
        covariance = mean((r-mr)*(w-mw) for r, w in zip(rs, ws))
        variance_r = mean((r-mr)**2 for r in rs)
        variance_w = mean((w-mw)**2 for w in ws)
        correlation = covariance / math.sqrt(variance_r*variance_w) if variance_r > 0 and variance_w > 0 else ""
        result.append(dict(zip(("run", "seed", "environment", "protection", "toxin_type", "birth_cohort", "exposed"), key)) |
                      dict(n=len(members), deaths=sum(r["outcome"] == "death" for r in members),
                           censored=sum(r["outcome"] == "censored" for r in members),
                           mean_resistance=mr, mean_logged_children=mw,
                           resistance_offspring_covariance=covariance,
                           descriptive_reproductive_differential=covariance/mw if mw > 0 else "",
                           descriptive_correlation=correlation))
    return result


def assess(root, ticks=10000, interval=100, require_current_terminal=False):
    summaries, _ = analyze(root, ticks, interval)
    root = Path(root)
    all_rows, sources, binaries, manifests = [], set(), set(), {}
    for run in sorted({r["run"] for r in summaries}):
        meta = json.loads((root / f"{run}.metadata.json").read_text(encoding="utf-8"))
        inputs = meta["inputs"]
        require(meta["status"] == "completed", f"{run}: incomplete metadata")
        require(inputs["ticks"] == ticks and inputs["interval"] == interval, f"{run}: metadata horizon")
        require(inputs["config_sha256"] == hashlib.sha256((root / f"{run}.json").read_bytes()).hexdigest(),
                f"{run}: config hash mismatch")
        config = json.loads((root / f"{run}.json").read_text(encoding="utf-8-sig"))
        require(inputs["seed"] == config["seed"], f"{run}: metadata seed mismatch")
        digest = hashlib.sha256(json.dumps(inputs["source_sha256"], sort_keys=True, separators=(",", ":")).encode()).hexdigest()
        sources.add(digest)
        binaries.add(inputs["binary_sha256"])
        manifests[run] = meta
        require(all((ROOT / p).is_file() and hashlib.sha256((ROOT / p).read_bytes()).hexdigest() == h
                    for p, h in inputs["source_sha256"].items()), f"{run}: recorded sources differ from current files")
        all_rows.extend(audit_run(root, run, ticks, interval))
    require(len(sources) == len(binaries) == 1, "Source/binary changes across runs")
    current_terminal = all(r["current_terminal_schema"] and
                           r["terminal_observation_phase"] in ("pre_removal_current_tick", "end_tick")
                           for r in all_rows)
    if require_current_terminal:
        require(current_terminal, "Current-tick terminal observations required")
    generations = summarize(all_rows, ("run", "seed", "environment", "protection", "toxin_type", "generation_depth"))
    cohorts = summarize(all_rows, ("run", "seed", "environment", "protection", "toxin_type", "birth_cohort"))
    run_types = summarize(all_rows, ("run", "seed", "environment", "protection", "toxin_type"))
    quality = dict(passed=True, runs=12, type_individual_rows=len(all_rows),
        biological_individuals=len(all_rows)//2, source_manifest_sha256=next(iter(sources)),
        binary_sha256=next(iter(binaries)), ticks=ticks, interval=interval,
        event_quality="births, deaths, paired types, terminals, lineage and population counts verified",
        current_tick_terminal=current_terminal,
        limits=["Death snapshots precede the death tick; exposure is a lower bound for deaths.",
                "EntityId generation is not biological generation.",
                "Only two seeds; descriptive ecosystem intervention, not proof of causal evolution.",
                "Cohorts and generations differ in age/environment/follow-up; mean offspring is descriptive."])
    if current_terminal:
        quality["limits"][0] = "Death observations are at pre-removal phase; the removal trigger does not identify causal toxin death."
    quality["input_sha256"] = {
        path.name: hashlib.sha256(path.read_bytes()).hexdigest()
        for run in sorted(manifests)
        for suffix in (".json", ".metadata.json", ".types.csv", ".births.csv", ".outcomes.csv", ".individuals.csv")
        for path in (root / f"{run}{suffix}",)
    }
    quality["analysis_sha256"] = {name: hashlib.sha256((ROOT / "tools" / name).read_bytes()).hexdigest()
                                   for name in ("analyze_phase0759.py", "analyze_phase0759_evolution.py")}
    return all_rows, generations, cohorts, run_types, compare_generations(generations), quality


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--ticks", type=int, default=10000)
    parser.add_argument("--interval", type=int, default=100)
    parser.add_argument("--require-current-terminal", action="store_true")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    try:
        individuals, generations, cohorts, runs, pairs, quality = assess(
            args.root, args.ticks, args.interval, args.require_current_terminal)
        components = generation_components(generations)
        associations = fitness_associations(individuals)
    except (ValueError, OSError, KeyError) as error:
        (args.out / "evolution_quality.json").write_text(json.dumps(dict(passed=False, error=str(error)), indent=2), encoding="utf-8")
        parser.exit(2, f"FAIL: {error}\n")
    for name, rows in (("individual_evolution", individuals), ("generations", generations),
                       ("birth_cohorts", cohorts), ("run_type_exposure_fitness", runs),
                       ("generation_paired_effects", pairs),
                       ("generation_components", components),
                       ("fitness_associations", associations)):
        write_csv(args.out / f"{name}.csv", rows)
    (args.out / "evolution_quality.json").write_text(json.dumps(quality, indent=2), encoding="utf-8")
    print(json.dumps({k: v for k, v in quality.items() if k not in ("input_sha256", "analysis_sha256")}, indent=2))


if __name__ == "__main__":
    main()
