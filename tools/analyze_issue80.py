"""Issue #80: judge diet_goal runs by the criteria fixed in Issue #80.

Usage: python tools/analyze_issue80.py DIR CONTROL_ARM [--seeds=90001,...]
Criteria (fixed 2026-11-20 before the run), each arm against CONTROL_ARM:
  1. Flee<->Pursue switches per 1000 herbivore Ticks at most half of the control (pooled)
  2. no large collapse:
     a. carnivores present in >= 0.7 of the 1000-Tick points of 10000..30000 in >= 6/8 seeds
     b. herbivores extinct in <= 2/8 seeds
     c. mean herbivores (10000..30000, pooled) 0.7-1.3 x the control
     d. mean carnivores (10000..30000, pooled) 0.5-2 x the control
     e. every run reaches 30000 Ticks
  3. in danger, the share of Ticks pursuing food is higher with reserve < 0.4 than >= 0.7
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from analyze_issue78 import fmt, mean, runs, summary  # noqa: E402


def risk(rows):
    last = rows[-1]
    get = lambda k: last.get(k) or 0  # noqa: E731
    return {
        "ticks": get("risk_herbivore_ticks"),
        "switches": get("risk_switches"),
        "danger": [get(f"risk_danger_ticks_{b}") for b in ("low", "mid", "high")],
        "pursue": [get(f"risk_pursue_in_danger_{b}") for b in ("low", "mid", "high")],
    }


def judge(found, arm, control, seeds):
    def pick(a):
        return [sd for sd in sorted(found[a]) if seeds is None or sd in seeds]

    sums = [summary(found[arm][sd]) for sd in pick(arm)]
    ctrl = [summary(found[control][sd]) for sd in pick(control)]
    r = [risk(found[arm][sd]) for sd in pick(arm)]
    rc = [risk(found[control][sd]) for sd in pick(control)]
    n = len(sums)
    rate = lambda rs: 1000 * sum(x["switches"] for x in rs) / max(1, sum(x["ticks"] for x in rs))  # noqa: E731
    switches, switches_ctrl = rate(r), rate(rc)
    present = sum(1 for s in sums if s["present"] is not None and s["present"] >= 0.7)
    extinct = sum(1 for s in sums if s["extinct"])
    herb = mean([s["herbivores"] for s in sums]) / mean([s["herbivores"] for s in ctrl])
    carn_ctrl = mean([s["carnivores"] for s in ctrl])
    carn = mean([s["carnivores"] for s in sums]) / carn_ctrl if carn_ctrl else None
    complete = all(s["ticks"] >= 30000 for s in sums)
    danger = [sum(x["danger"][b] for x in r) for b in range(3)]
    pursue = [sum(x["pursue"][b] for x in r) for b in range(3)]
    share = [pursue[b] / danger[b] if danger[b] else None for b in range(3)]
    c = {
        "1": switches <= 0.5 * switches_ctrl,
        "2a": present >= math.ceil(n * 3 / 4),
        "2b": extinct <= n // 4,
        "2c": 0.7 <= herb <= 1.3,
        "2d": carn is not None and 0.5 <= carn <= 2,
        "2e": complete,
        "3": share[0] is not None and share[2] is not None and share[0] > share[2],
    }
    return {
        "n": n, "switches": switches, "switches_ctrl": switches_ctrl, "present": present,
        "mean_present": mean([s["present"] for s in sums]), "extinct": extinct, "herb": herb,
        "carn": carn, "share": share, "c": c,
    }


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    seeds = None
    for a in sys.argv[1:]:
        if a.startswith("--seeds="):
            seeds = {int(x) for x in a.split("=", 1)[1].split(",")}
    found = runs(args[0])
    control = args[1]
    print("| 条件 | 切り替え（草食1000 Tickあたり） | 対照 | 1 | 肉食がいた割合（平均） | ≥0.7 の seed | 2a | 草食の絶滅 | 2b | 草食の数（対照比） | 2c | 肉食の数（対照比） | 2d | 2e | 危険の中で植物へ（蓄え低/中/高） | 3 | 合格 |")
    print("|" + "---|" * 17)
    for arm in sorted(found):
        j = judge(found, arm, control, seeds)
        c = j["c"]
        ok = all(c.values()) if arm != control else None
        share = "/".join(fmt(x, 3) for x in j["share"])
        print(
            f"| {arm} | {j['switches']:.1f} | {j['switches_ctrl']:.1f} | {fmt(c['1']) if arm != control else '—'} | "
            f"{fmt(j['mean_present'])} | {j['present']}/{j['n']} | {fmt(c['2a'])} | {j['extinct']}/{j['n']} | {fmt(c['2b'])} | "
            f"{fmt(j['herb'])} | {fmt(c['2c']) if arm != control else '—'} | {fmt(j['carn'])} | {fmt(c['2d']) if arm != control else '—'} | "
            f"{fmt(c['2e'])} | {share} | {fmt(c['3']) if arm != control else '—'} | {fmt(ok) if ok is not None else '（対照）'} |"
        )


if __name__ == "__main__":
    main()
