"""Issue #76: rank conditions judged on 8 seeds by the rule fixed on 2026-11-20.

Usage: python tools/rank_issue76.py DIR [DIR ...]
Each DIR holds mimicry_goal CSVs (<arm>_<seed>.csv). Only arms with 8 seeds are ranked.
Rule (fixed before the round-9 results, Issue #76):
  1. number of the goal criteria P1, P2, P3 and the revised M1 that are met
  2. tie: sum over the four of min(1, seeds met / seeds required) (P3: 1 if met, else 0)
  3. tie: number of seeds meeting M2
Arms without learning (_NL) and runs without choice-scene columns cannot meet M1.
"""
import math
import os
import re
import sys

sys.path.insert(0, os.path.dirname(__file__))
import analyze_issue76 as a  # noqa: E402


def score(found, arm):
    seeds = sorted(found[arm])
    n = len(seeds)
    need = math.ceil(n * 3 / 4)
    control = re.sub(r"_G\d+$", "_NL", arm)
    nl = found.get(control) if control != arm else None
    flags = []
    sums = []
    for sd in seeds:
        s = a.summary(found[arm][sd])
        sums.append(s)
        c = a.summary(nl[sd]) if nl and sd in nl else None
        flags.append(a.judge(s, c))
    count = lambda k: sum(f[k] for f in flags)
    extinct = sum(s["herb_extinct"] for s in sums)
    present = a.mean([s["carn_present"] for s in sums])
    p3 = extinct <= n // 4 and present is not None and present >= 0.7
    met = [count("P1") >= need, count("P2") >= need, p3, count("M1new") >= need]
    closeness = (
        min(1, count("P1") / need)
        + min(1, count("P2") / need)
        + (1 if p3 else 0)
        + min(1, count("M1new") / need)
    )
    return {
        "arm": arm,
        "met": sum(met),
        "closeness": closeness,
        "m2": count("M2") if nl else 0,
        "detail": f"P1 {count('P1')}/{n} P2 {count('P2')}/{n} P3 {'○' if p3 else '×'} "
        f"M1 {count('M1new')}/{n} M2 {count('M2') if nl else '—'}/{n}",
    }


def main():
    rows = []
    for d in sys.argv[1:]:
        found = a.runs(d)
        for arm in found:
            if len(found[arm]) != 8 or arm.endswith("_NL") or arm == "std":
                continue
            r = score(found, arm)
            r["dir"] = d
            rows.append(r)
    rows.sort(key=lambda r: (-r["met"], -r["closeness"], -r["m2"]))
    print("| 順位 | 条件 | 実行 | 満たした数 | 近さ | 内訳 |")
    print("|---|---|---|---|---|---|")
    for i, r in enumerate(rows, 1):
        run = "/".join(r["dir"].rstrip("/").split("/")[-3:-1])
        print(f"| {i} | {r['arm']} | {run} | {r['met']} | {r['closeness']:.2f} | {r['detail']} |")


if __name__ == "__main__":
    main()
