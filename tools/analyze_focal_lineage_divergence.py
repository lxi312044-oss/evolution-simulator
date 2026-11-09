"""Decompose the two contrasting toxin-on focal lineages from Issue 35."""
import gzip
import json
from collections import Counter
from pathlib import Path
from statistics import mean

from run_metadata import sha256

SOURCE = Path("results/focal-intervention-longterm-v3-20261108")
OUTPUT = Path("results/focal-lineage-decomposition-20261109.json")


def summarize(path):
    focal = None
    descendants = {}
    generations, windows, branches = Counter(), Counter(), Counter()
    direct, removed, surviving = [], {}, set()
    with gzip.open(path, "rt", encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line)
            kind = row["kind"]
            if kind == "intervention_audit":
                focal = tuple(row["receipt"]["id"])
            elif kind == "birth":
                parent, child = tuple(row["parent"]), tuple(row["id"])
                if parent == focal:
                    direct.append(row["tick"])
                    branch, depth = child, 1
                elif parent in descendants:
                    _, branch, parent_depth = descendants[parent]
                    depth = parent_depth + 1
                else:
                    continue
                descendants[child] = row["tick"], branch, depth
                generations[depth] += 1
                windows[(row["tick"] - 1) // 1000 + 1] += 1
                branches[branch] += 1
            elif kind in ("removal", "survivor"):
                ident = tuple(row["id"])
                if ident in descendants:
                    if kind == "removal":
                        removed[ident] = row["tick"]
                    else:
                        surviving.add(ident)
    if set(descendants) != set(removed) | surviving or set(removed) & surviving:
        raise ValueError("Descendant terminal records do not reconcile")
    return dict(direct_birth_ticks=direct, descendants=len(descendants),
                births_by_generation=dict(sorted(generations.items())),
                births_by_1000_tick_window=dict(sorted(windows.items())),
                branches=[dict(child=list(child), births=count)
                          for child, count in branches.most_common()],
                first_birth=min(direct), last_birth=max(t for t, _, _ in descendants.values()),
                last_removal=max(removed.values(), default=None),
                present_at_end=len(surviving),
                mean_completed_persistence=mean(t - descendants[i][0] for i, t in removed.items()))


def main():
    execution = json.loads((SOURCE / "execution.json").read_text(encoding="utf-8"))
    if execution["status"] != "completed":
        raise ValueError("Incomplete source run")
    runs = {}
    for seed in (12350, 12351):
        for resistance in (0.2, 0.8):
            name = f"seed{seed}_toxin1_r{resistance}"
            raw = SOURCE / f"{name}.jsonl.gz"
            summary = SOURCE / f"{name}.summary.json"
            for path in (raw, summary):
                if sha256(path) != execution["output_sha256"][path.name]:
                    raise ValueError(f"Source hash mismatch: {path}")
            result = summarize(raw)
            expected = json.loads(summary.read_text(encoding="utf-8"))
            if (result["descendants"], len(result["direct_birth_ticks"])) != (
                expected["focal_descendant_births"], expected["focal_direct_births"]
            ):
                raise ValueError(f"Birth totals mismatch: {name}")
            runs[name] = result
    with OUTPUT.open("x", encoding="utf-8") as stream:
        json.dump(dict(source_execution_sha256=sha256(SOURCE / "execution.json"), runs=runs),
                  stream, indent=2)


if __name__ == "__main__":
    main()
