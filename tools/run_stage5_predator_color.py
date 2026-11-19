"""Execute and audit the fixed Issue #39 confirmation without changing the ecology."""
import argparse
import csv
import gzip
import hashlib
import json
import math
import os
import subprocess
import time
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

import numpy as np
import scipy
from scipy.stats import t

from analyze_stage5_predator_color import CHANNELS, RCOND, SEEDS, analyze

OUTPUT = Path("results/issue39-predator-color-confirmation")
BINARY = Path("target/release/examples") / (
    "stage5_predator_color.exe" if os.name == "nt" else "stage5_predator_color"
)
CONFIG = Path("config/stage4-inherited-predator-candidate.json")
CONFIG_HASH = "e2e366b1a7bdcf36826972898592f1e3062e18ac008f143767f0321de808ffcc"
PER_ARM = 2 * 1024**2
LIMIT = 544 * 1024**2
TRACKED = (
    CONFIG, Path("Cargo.lock"),
    Path("crates/sim-core/examples/stage5_predator_color.rs"),
    Path(__file__), Path("tools/analyze_stage5_predator_color.py"),
    Path("tools/test_stage5_predator_color.py"), Path("tools/test_stage5_predator_execution.py"),
)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check(condition, message):
    if not condition:
        raise ValueError(message)


def audit(data, seed, arm, end):
    check((data["seed"], data["on"], data["tick"]) == (seed, arm == "on", end), "run identity")
    check(data["audit_points"] == list(range(0, end + 1, 500)), "audit checkpoints")
    check(data["max_population"] <= 5000, "capacity")
    check(all(math.isfinite(data[k]) and data[k] < 0.001 for k in (
        "max_n_error", "max_e_error", "max_toxin_error")), "conservation")
    founders = {tuple(f["id"]): f for f in data["founders"]}
    check(len(founders) == len(data["founders"]) == 80, "founder IDs")
    lineage = {identity: (identity, 0) for identity in founders}
    living = set(founders)
    descendants = Counter({identity: 0 for identity in founders})
    by_depth = Counter()
    births = defaultdict(list)
    removals = defaultdict(list)
    for event in data["births"]:
        births[event["tick"]].append(event)
    for event in data["removals"]:
        removals[event["tick"]].append(event)
    check(all(0 < tick <= end for tick in births.keys() | removals.keys()), "event horizon")
    samples = {s["tick"]: s for s in data["samples"]}
    check(list(samples) == data["audit_points"], "population checkpoints")
    check(samples[0]["herbivores"] == 80 and samples[0]["births"] == samples[0]["removals"] == 0,
          "initial population counts")
    born = removed = 0
    for tick in sorted(births.keys() | removals.keys() | samples.keys()):
        if tick == 0:
            continue
        # Herbivore physiology and removal precede herbivore reproduction.
        for event in removals[tick]:
            identity = tuple(event["id"])
            check(identity in living, f"duplicate/unknown removal {tick} {identity}")
            check(event["reason"] in ("age_limit", "hp_nonpositive"), "unknown removal trigger")
            living.remove(identity)
            removed += 1
        for event in births[tick]:
            identity, parent = tuple(event["id"]), tuple(event["parent"])
            check(identity not in lineage and parent in living, f"invalid parent/child {tick}")
            check(len(event["rgb8"]) == 3 and all(0 <= x <= 255 for x in event["rgb8"]), "birth RGB")
            root, depth = lineage[parent]
            lineage[identity] = (root, depth + 1)
            living.add(identity)
            descendants[root] += 1
            by_depth[depth + 1] += 1
            born += 1
        if tick in samples:
            point = samples[tick]
            check((born, removed, len(living)) == (point["births"], point["removals"], point["herbivores"]),
                  f"population mismatch at {tick}")
    survivors = {tuple(i) for i in data["survivors"]}
    check(survivors == living and len(survivors) == len(data["survivors"]), "terminal ID ledger")
    check(sum(descendants.values()) == born == len(data["births"]), "root totals")
    return founders, descendants, by_depth


def run_condition(seed, arm, deadline):
    row = {"seed": seed, "arm": arm}
    remaining = min(120, deadline - time.monotonic())
    if remaining <= 0:
        return {**row, "error": "12-hour deadline"}
    try:
        process = subprocess.run([str(BINARY), str(seed), arm], capture_output=True,
                                 check=True, timeout=remaining)
        data = json.loads(process.stdout)
        audit(data, seed, arm, 6000)
        packed = gzip.compress(process.stdout, compresslevel=6, mtime=0)
        check(len(packed) <= PER_ARM, "2 MiB compressed arm limit")
        check(time.monotonic() <= deadline, "12-hour deadline")
        path = OUTPUT / f"{seed}-{arm}.json.gz"
        with path.open("xb") as stream:
            stream.write(packed)
        digest = hashlib.sha256(packed).hexdigest()
        check(sha(path) == digest, "saved file hash mismatch")
        return {**row, "file": path.name, "bytes": len(packed), "sha256": digest,
                "seconds": data["seconds"]}
    except (ValueError, KeyError, OSError, subprocess.SubprocessError) as exc:
        stderr = getattr(exc, "stderr", None) or b""
        return {**row, "error": repr(exc), "stderr": stderr.decode(errors="replace")[-1500:]}


def adjusted_slopes(founders, difference):
    colors = np.array([f["rgb8"] for f in founders]) / 255.0
    colors = (colors - colors.mean(axis=0)) / 0.1
    attributes = np.array([
        [*f["load"], *f["stock"], *f["resistance"], f["x"], f["y"], f["age"],
         f["hp"], f["e"], f["body_n"], f["reserve_n"]] for f in founders
    ])
    vary = np.ptp(attributes, axis=0) > 0
    attributes = attributes[:, vary]
    covariates = np.column_stack((np.ones(len(founders)),
                                 (attributes - attributes.mean(axis=0)) / attributes.std(axis=0)))
    outputs = np.column_stack((colors, difference))
    fitted = covariates @ np.linalg.lstsq(covariates, outputs, rcond=RCOND)[0]
    residual = outputs - fitted
    beta, _, rank, _ = np.linalg.lstsq(residual[:, :3], residual[:, 3], rcond=RCOND)
    return beta.tolist() if rank == 3 and np.all(np.isfinite(beta)) else None


def save_json(path, data):
    text = json.dumps(data, ensure_ascii=False, allow_nan=False, indent=2)
    payload = text.encode("utf-8")
    used = sum(p.stat().st_size for p in path.parent.iterdir() if p.is_file())
    check(used + len(payload) <= LIMIT, "544 MiB aggregate output limit")
    with path.open("xb") as stream:
        stream.write(payload)


def collect(manifest):
    by_arm = {(row["seed"], row["arm"]): row for row in manifest["conditions"]}
    paired = []
    supplement = {}
    failures = {}
    completed = []
    for seed in SEEDS:
        arms = {}
        for arm in ("on", "off"):
            row = by_arm[seed, arm]
            if "error" in row:
                failures[f"{seed}-{arm}"] = row["error"]
                continue
            try:
                path = OUTPUT / row["file"]
                check(sha(path) == row["sha256"], "archive hash mismatch")
                with gzip.open(path, "rt", encoding="utf-8") as stream:
                    data = json.load(stream)
                arms[arm] = (data, *audit(data, seed, arm, 6000))
                completed.append(data)
            except (ValueError, KeyError, OSError) as exc:
                failures[f"{seed}-{arm}"] = str(exc)
        if len(arms) != 2:
            continue
        on, founders, on_counts, on_depth = arms["on"]
        off, off_founders, off_counts, off_depth = arms["off"]
        if on["initial_hash"] != off["initial_hash"] or founders != off_founders:
            failures[str(seed)] = "paired initial state/RNG/founders mismatch"
            continue
        ids = sorted(founders)
        for identity in ids:
            paired.append(dict(seed=seed, index=identity[0], generation=identity[1],
                               **dict(zip(CHANNELS, founders[identity]["rgb8"])),
                               on_births=on_counts[identity], off_births=off_counts[identity]))
        diff = [on_counts[i] - off_counts[i] for i in ids]
        attrs = [founders[i] for i in ids]
        color = np.array([f["rgb8"] for f in attrs])
        attributes = np.array([
            [*f["load"], *f["stock"], *f["resistance"], f["x"], f["y"], f["age"],
             f["hp"], f["e"], f["body_n"], f["reserve_n"]] for f in attrs
        ])
        correlations = []
        for column in attributes.T:
            correlations.append([
                float(np.corrcoef(component, column)[0, 1]) if np.ptp(column) and np.ptp(component)
                else None for component in color.T
            ])
        supplement[seed] = {
            "adjusted_slopes_rgb": adjusted_slopes(attrs, diff),
            "initial_attribute_correlations_rgb": correlations,
            "on_mean": sum(on_counts.values()) / 80, "off_mean": sum(off_counts.values()) / 80,
            "on_zero": sum(v == 0 for v in on_counts.values()),
            "off_zero": sum(v == 0 for v in off_counts.values()),
            "on_generations": on_depth, "off_generations": off_depth,
            "on_predator_windows": on["windows"], "off_predator_windows": off["windows"],
        }
    primary = analyze(paired)
    save_json(OUTPUT / "primary.json", primary)
    adjusted = [s["adjusted_slopes_rgb"] for s in supplement.values()
                if s["adjusted_slopes_rgb"] is not None]
    adjusted_intervals = None
    if len(adjusted) >= 2:
        values = np.array(adjusted)
        means = values.mean(axis=0)
        widths = t.ppf(1 - 0.05 / 6, len(values) - 1) * values.std(axis=0, ddof=1) / np.sqrt(len(values))
        adjusted_intervals = np.column_stack((means, means - widths, means + widths)).tolist()
    checks = {key: max((d[key] for d in completed), default=None)
              for key in ("max_population", "max_n_error", "max_e_error", "max_toxin_error")}
    save_json(OUTPUT / "supplement.json", {
        "completed_arms": len(completed), "paired_seeds": len(supplement), "failures": failures,
        "audit_maxima": checks, "adjusted_valid_seeds": len(adjusted),
        "adjusted_intervals_rgb": adjusted_intervals,
        "initial_attribute_order": ["load_a", "load_b", "stock_a", "stock_b", "resistance_a",
                                    "resistance_b", "x", "y", "age", "hp", "e", "body_n", "reserve_n"],
        "per_seed": supplement,
    })
    with (OUTPUT / "paired_founders.csv").open("x", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=("seed", "index", "generation", *CHANNELS,
                                                   "on_births", "off_births"))
        writer.writeheader()
        writer.writerows(paired)
    check(sum(p.stat().st_size for p in OUTPUT.iterdir() if p.is_file()) <= LIMIT, "total output limit")
    print(json.dumps({"decision": primary["decision"], "completed": len(completed),
                      "valid_seeds": primary["valid_seeds"], "failures": failures,
                      "intervals": primary["intervals"]}), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--preflight", action="store_true")
    args = parser.parse_args()
    check(sha(CONFIG) == CONFIG_HASH, "fixed config hash")
    if args.preflight:
        hashes = set()
        for arm in ("on", "off"):
            output = subprocess.check_output([str(BINARY), "12345", arm], timeout=120)
            data = json.loads(output)
            _, descendants, _ = audit(data, 12345, arm, 2000)
            hashes.add(data["initial_hash"])
            print(arm, "births", sum(descendants.values()), "audits", data["audit_points"], flush=True)
        check(len(hashes) == 1, "preflight initial equality")
        return
    check(not subprocess.check_output(["git", "status", "--porcelain", "--", *map(str, TRACKED)]),
          "the fixed inputs, runner and tests must be committed")
    check(not subprocess.check_output(["git", "status", "--porcelain", "--", "crates/sim-core/src"]),
          "ecology source must be clean")
    OUTPUT.mkdir()
    start = time.monotonic()
    deadline = start + 12 * 60**2
    manifest = {
        "commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "binary_sha256": sha(BINARY), "input_sha256": {p.as_posix(): sha(p) for p in TRACKED},
        "numpy": np.__version__, "scipy": scipy.__version__,
        "time_limit_seconds": 43200, "arm_seconds": 120, "parallelism": 4,
        "output_limit_bytes": LIMIT, "conditions": [],
    }
    save_json(OUTPUT / "start.json", manifest)
    with ThreadPoolExecutor(max_workers=4) as pool:
        tasks = [pool.submit(run_condition, seed, arm, deadline)
                 for seed in SEEDS for arm in ("on", "off")]
        with (OUTPUT / "progress.jsonl").open("x", encoding="utf-8") as progress:
            for task in as_completed(tasks):
                result = task.result()
                manifest["conditions"].append(result)
                line = json.dumps(result, ensure_ascii=False)
                progress.write(line + "\n")
                progress.flush()
                print(f"{len(manifest['conditions'])}/256 {line}", flush=True)
    manifest["conditions"].sort(key=lambda r: (r["seed"], r["arm"]))
    manifest["execution_seconds"] = time.monotonic() - start
    save_json(OUTPUT / "manifest.json", manifest)
    collect(manifest)


if __name__ == "__main__":
    main()
