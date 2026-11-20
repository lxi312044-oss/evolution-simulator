"""Issue #82 stage 2: judge diet_goal runs by the criteria fixed in Issue #82.

Usage: python tools/analyze_issue82.py DIR CONTROL_ARM [--seeds=...]
Criteria (fixed 2026-11-20 before the run), each arm against CONTROL_ARM (values observed only):
  1. no large collapse: carnivores present in >= 0.7 of 10000..30000 in >= 6/8 seeds, herbivores
     extinct in <= 2/8 seeds, mean herbivores 0.7-1.3 x and mean carnivores 0.5-2 x the control
     (10000..30000, pooled), every run reaches 30000 Ticks
  2. mean carnivore tendency over 10000..30000 (record points, pooled) at least 0.02 above the
     control
  3. carnivores bite carnivores (cumulative bites > 0) in >= 6/8 seeds
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from analyze_issue78 import W0, W1, fmt, mean, runs, summary  # noqa: E402


def tendency(rows):
    return mean([r.get("apex_carnivore_mean") for r in rows if W0 <= r["tick"] <= W1])


def judge(found, arm, control, seeds):
    def pick(a):
        return [sd for sd in sorted(found[a]) if seeds is None or sd in seeds]

    sums = [summary(found[arm][sd]) for sd in pick(arm)]
    ctrl = [summary(found[control][sd]) for sd in pick(control)]
    n = len(sums)
    present = sum(1 for s in sums if s["present"] is not None and s["present"] >= 0.7)
    extinct = sum(1 for s in sums if s["extinct"])
    herb = mean([s["herbivores"] for s in sums]) / mean([s["herbivores"] for s in ctrl])
    carn_ctrl = mean([s["carnivores"] for s in ctrl])
    carn = mean([s["carnivores"] for s in sums]) / carn_ctrl if carn_ctrl else None
    t = mean([tendency(found[arm][sd]) for sd in pick(arm)])
    t_ctrl = mean([tendency(found[control][sd]) for sd in pick(control)])
    bites = [found[arm][sd][-1].get("apex_bites") or 0 for sd in pick(arm)]
    c = {
        "1": present >= math.ceil(n * 3 / 4) and extinct <= n // 4 and 0.7 <= herb <= 1.3
        and carn is not None and 0.5 <= carn <= 2 and all(s["ticks"] >= 30000 for s in sums),
        "2": t is not None and t_ctrl is not None and t - t_ctrl >= 0.02,
        "3": sum(1 for b in bites if b > 0) >= math.ceil(n * 3 / 4),
    }
    return {"n": n, "present": present, "mean_present": mean([s["present"] for s in sums]),
            "extinct": extinct, "herb": herb, "carn": carn, "t": t, "t_ctrl": t_ctrl,
            "bites": bites, "c": c}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    seeds = None
    for a in sys.argv[1:]:
        if a.startswith("--seeds="):
            seeds = {int(x) for x in a.split("=", 1)[1].split(",")}
    found = runs(args[0])
    control = args[1]
    print("| 条件 | 肉食がいた割合（平均） | ≥0.7 の seed | 草食の絶滅 | 草食の数（対照比） | 肉食の数（対照比） | 1 | 肉食の傾向の平均 | 対照 | 2 | 肉食へのかみつき（seed ごと） | 3 | 合格 |")
    print("|" + "---|" * 13)
    for arm in sorted(found):
        if arm != control and not arm.startswith(control.rsplit("_", 1)[0]):
            continue
        j = judge(found, arm, control, seeds)
        c = j["c"]
        is_ctrl = arm == control
        ok = None if is_ctrl else all(c.values())
        print(
            f"| {arm} | {fmt(j['mean_present'])} | {j['present']}/{j['n']} | {j['extinct']}/{j['n']} | "
            f"{fmt(j['herb'])} | {fmt(j['carn'])} | {'—' if is_ctrl else fmt(c['1'])} | {fmt(j['t'], 3)} | "
            f"{fmt(j['t_ctrl'], 3)} | {'—' if is_ctrl else fmt(c['2'])} | {' '.join(str(int(b)) for b in j['bites'])} | "
            f"{'—' if is_ctrl else fmt(c['3'])} | {'（対照）' if is_ctrl else fmt(ok)} |"
        )


if __name__ == "__main__":
    main()
