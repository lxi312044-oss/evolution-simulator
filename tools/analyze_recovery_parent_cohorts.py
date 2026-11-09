"""Decompose recovery births by high-resistance parents present at the trough."""
import argparse
import json
from pathlib import Path

from run_confirmation import require
from run_longterm_resistance import resistance_bin
from run_metadata import sha256


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def read_run(path):
    lives, samples, births = {}, {}, []
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line)
            kind = row["kind"]
            if kind == "initial":
                min_age = row["config"]["herbivores"]["reproduction_min_age"]
                for animal in row["founders"]:
                    lives[tuple(animal["id"])] = (-animal["age"], None, resistance_bin(animal["resistance"]))
            elif kind == "birth":
                child, parent = tuple(row["id"]), tuple(row["parent"])
                require(child not in lives and parent in lives, "Broken parent/child identity")
                lives[child] = (row["tick"], None, resistance_bin(row["resistance"]))
                births.append((row["tick"], parent, lives[child][2]))
            elif kind == "removal":
                ident = tuple(row["id"])
                start, end, group = lives[ident]
                require(end is None and start <= row["tick"], "Invalid removal")
                lives[ident] = (start, row["tick"], group)
            elif kind == "population":
                samples[row["tick"]] = row["counts"][2]
    return lives, births, samples, min_age


def summarize(run, start, end):
    lives, births, samples, min_age = run
    cohort = {ident for ident, (born, removed, group) in lives.items()
              if group == 2 and born <= start and (removed is None or removed > start)}
    require(len(cohort) == samples[start], f"Cannot identify positive-HP trough cohort at {start}")
    mature = {ident for ident in cohort if start - lives[ident][0] >= min_age}
    age_reached = {ident for ident in cohort - mature
                   if lives[ident][0] + min_age <= end
                   and (lives[ident][1] is None or lives[ident][0] + min_age < lives[ident][1])}
    rows = [(parent, group) for tick, parent, group in births if start < tick <= end]
    offspring = [(parent, group) for parent, group in rows if parent in cohort]
    new_high_parents = [(parent, group) for parent, group in rows
                        if lives[parent][0] > start and lives[parent][2] == 2]
    return dict(start=start, end=end, min_age=min_age, trough_high=len(cohort),
                age_eligible_at_trough=len(mature), reach_age_during_window=len(age_reached),
                trough_high_breeders=len({parent for parent, _ in offspring}),
                trough_high_offspring=len(offspring),
                high_offspring_from_trough_high=sum(group == 2 for _, group in offspring),
                offspring_from_new_high_parents=len(new_high_parents),
                high_offspring_from_new_high_parents=sum(group == 2 for _, group in new_high_parents),
                high_offspring_from_other_parents=sum(group == 2 and lives[parent][2] != 2
                                                       for parent, group in rows),
                all_high_births=sum(group == 2 for _, group in rows))


def main(observed, control, output):
    assessment = load(control / "assessment.json")
    results = {"same_phase": {}, "same_duration": {}, "same_tick": {}}
    for name, pair in assessment["pairs"]["same_phase"].items():
        paths = {"on": observed / f"{name}.jsonl", "off": control / f"{name}.jsonl"}
        for arm, path in paths.items():
            require(sha256(path) == assessment["runs"][name][f"{arm}_quality"]["log_sha256"],
                    "Source log changed")
        runs = {arm: read_run(path) for arm, path in paths.items()}
        duration = min(row["end"]["tick"] - row["start"]["tick"] for row in pair.values())
        for mode in results:
            windows = assessment["pairs"]["same_tick"][name] if mode == "same_tick" else pair
            results[mode][name] = {}
            for arm, row in windows.items():
                start = row["start"]["tick"]
                end = start + duration if mode == "same_duration" else row["end"]["tick"]
                summary = summarize(runs[arm], start, end)
                require(summary["all_high_births"] ==
                        sum(summary[k] for k in ("high_offspring_from_trough_high",
                                                 "high_offspring_from_new_high_parents",
                                                 "high_offspring_from_other_parents")),
                        "High births do not reconcile")
                if mode != "same_duration":
                    require(summary["all_high_births"] == row["change"]["high_births"],
                            "High births differ from audited control results")
                results[mode][name][arm] = summary
    with output.open("x", encoding="utf-8") as stream:
        json.dump(results, stream, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("observed", type=Path)
    parser.add_argument("control", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    main(args.observed, args.control, args.output)
