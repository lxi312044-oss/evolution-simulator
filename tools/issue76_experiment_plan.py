"""Issue #76: turn an experiment request into GitHub Actions matrices.

Usage: python tools/issue76_experiment_plan.py experiment/request.json
Prints `builds=...`, `runs=...` and `check=...` lines for $GITHUB_OUTPUT.

Request format (JSON):
{
  "name": "r0-diagnosis",            # results folder name
  "check": true,                       # run fmt/clippy/tests for sim-core first
  "builds": [{"id": "new", "sha": "", "example": "mimicry_goal"}],
  "runs": [{"build": "new", "arms": ["std"], "seeds": [90006], "ticks": 30000,
            "extra": ["500"]}],
  "collect_from_run": ""               # optional: only re-collect the artifacts of that run
}
An empty sha means the pushed commit. Each run entry becomes one job per arm; the seeds of
that arm run in parallel inside the job. The example CLI is `<example> <arm> <seed> <ticks>
[extra...]` with CSV on stdout.
"""
import json
import re
import sys

NAME = re.compile(r"^[A-Za-z0-9_.-]+$")


def plan(request):
    builds = []
    ids = set()
    for b in request["builds"]:
        for key in ("id", "example"):
            if not NAME.match(b[key]):
                raise ValueError(f"bad {key}: {b[key]!r}")
        if b.get("sha") and not re.fullmatch(r"[0-9a-f]{40}", b["sha"]):
            raise ValueError(f"sha must be a full commit id: {b['sha']!r}")
        ids.add(b["id"])
        builds.append({"id": b["id"], "sha": b.get("sha", ""), "example": b["example"]})
    runs = []
    for r in request["runs"]:
        if r["build"] not in ids:
            raise ValueError(f"unknown build {r['build']!r}")
        example = next(b["example"] for b in builds if b["id"] == r["build"])
        seeds = [int(s) for s in r["seeds"]]
        ticks = int(r["ticks"])
        if not 100 <= ticks <= 100000:
            raise ValueError("ticks out of range")
        extra = [str(int(x)) for x in r.get("extra", [])]
        for arm in r["arms"]:
            if not NAME.match(arm):
                raise ValueError(f"bad arm {arm!r}")
            runs.append(
                {
                    "build": r["build"],
                    "example": example,
                    "arm": arm,
                    "seeds": " ".join(map(str, seeds)),
                    "ticks": ticks,
                    "extra": " ".join(extra),
                }
            )
    if not NAME.match(request["name"]):
        raise ValueError("bad name")
    collect_from = str(request.get("collect_from_run", ""))
    if collect_from and not collect_from.isdigit():
        raise ValueError("collect_from_run must be a run id")
    return builds, runs, bool(request.get("check", False)), collect_from


def main():
    with open(sys.argv[1], encoding="utf-8") as f:
        builds, runs, check, collect_from = plan(json.load(f))
    print("builds=" + json.dumps(builds, separators=(",", ":")))
    print("runs=" + json.dumps(runs, separators=(",", ":")))
    print("check=" + ("true" if check else "false"))
    print("collect_from=" + collect_from)


if __name__ == "__main__":
    main()
