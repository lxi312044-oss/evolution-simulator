"""Issue #78 stage 2: judge diet_goal runs by the criteria fixed in Issue #78.

Usage: python tools/analyze_issue78.py DIR [--seeds=90001,90002,...]
DIR holds diet_goal CSVs (<arm>_<seed>.csv).
Criteria (fixed 2026-11-20 before the run):
  1. carnivores present in >= 0.7 of the 1000-Tick points of 10000..30000 in >= 6/8 seeds
  2. herbivores extinct (0 at any record) in <= 2/8 seeds
  3. pooled over seeds, the mean parent value of herbivore-to-carnivore births is at least
     0.1 above the mean herbivore value (mean over all record points of 0..30000)
"""
import csv
import glob
import math
import os
import re
import sys

W0, W1 = 10000, 30000


def num(x):
    try:
        v = float(x)
    except (TypeError, ValueError):
        return None
    return v if math.isfinite(v) else None


def load(path):
    with open(path, encoding="utf-8") as f:
        return [{k: (v if k == "arm" else num(v)) for k, v in r.items()} for r in csv.DictReader(f)]


def runs(directory):
    found = {}
    for path in glob.glob(os.path.join(directory, "*.csv")):
        m = re.match(r"(.+)_(\d+)\.csv$", os.path.basename(path))
        if m:
            found.setdefault(m.group(1), {})[int(m.group(2))] = load(path)
    return found


def mean(values):
    values = [v for v in values if v is not None]
    return sum(values) / len(values) if values else None


def summary(rows):
    w = [r for r in rows if W0 <= r["tick"] <= W1 and r["tick"] % 1000 == 0]
    last = rows[-1]
    s = {
        "ticks": last["tick"],
        "present": mean([1.0 if r["carnivores"] > 0 else 0.0 for r in w]),
        "extinct": any(r["herbivores"] == 0 for r in rows),
        "herbivores": mean([r["herbivores"] for r in w]),
        "carnivores": mean([r["carnivores"] for r in w]),
        "hc": last.get("births_herbivore_to_carnivore"),
        "cc": last.get("births_carnivore_to_carnivore"),
        "ch": last.get("births_carnivore_to_herbivore"),
        "born_from_herbivores": last["carnivores_born_from_herbivores"],
        "herb_value": mean([r.get("herbivore_diet_mean") for r in rows]),
        "herb_value_h1": mean([r.get("herbivore_diet_mean") for r in w if r["tick"] < 20000]),
        "herb_value_h2": mean([r.get("herbivore_diet_mean") for r in w if r["tick"] >= 20000]),
        "carn_value_h1": mean([r.get("carnivore_diet_mean") for r in w if r["tick"] < 20000]),
        "carn_value_h2": mean([r.get("carnivore_diet_mean") for r in w if r["tick"] >= 20000]),
        "intermediate": mean([r.get("intermediate_share") for r in w]),
        "parent_n": last.get("parent_herbivore_to_carnivore_n"),
        "parent_mean": last.get("parent_herbivore_to_carnivore_mean"),
    }
    return s


def fmt(x, nd=2):
    if x is None:
        return "—"
    if isinstance(x, bool):
        return "○" if x else "×"
    if float(x).is_integer() and abs(x) >= 1:
        return str(int(x))
    return f"{x:.{nd}f}"


def judge(sums):
    n = len(sums)
    need = math.ceil(n * 3 / 4)
    present = sum(1 for s in sums if s["present"] is not None and s["present"] >= 0.7)
    extinct = sum(1 for s in sums if s["extinct"])
    pn = sum(s["parent_n"] or 0 for s in sums)
    psum = sum((s["parent_n"] or 0) * (s["parent_mean"] or 0) for s in sums)
    parent = psum / pn if pn else None
    herb = mean([s["herb_value"] for s in sums])
    gap = parent - herb if parent is not None and herb is not None else None
    c1 = present >= need
    c2 = extinct <= n // 4
    c3 = gap is not None and gap >= 0.1
    return {
        "n": n,
        "present_seeds": present,
        "extinct": extinct,
        "parent": parent,
        "herb": herb,
        "gap": gap,
        "c": (c1, c2, c3),
        "mean_present": mean([s["present"] for s in sums]),
    }


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    seeds = None
    for a in sys.argv[1:]:
        if a.startswith("--seeds="):
            seeds = {int(x) for x in a.split("=", 1)[1].split(",")}
    found = runs(args[0])
    print("| 条件 | seed数 | 肉食がいた割合（平均） | ≥0.7 の seed | 草食の絶滅 | 草食→肉食の親の値 | 草食全体の値 | 差 | 1 | 2 | 3 | 合格 |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for arm in sorted(found):
        sums = [summary(found[arm][sd]) for sd in sorted(found[arm]) if seeds is None or sd in seeds]
        j = judge(sums)
        c1, c2, c3 = j["c"]
        ok = c1 and c2 and c3 if arm != "std" else None
        print(
            f"| {arm} | {j['n']} | {fmt(j['mean_present'])} | {j['present_seeds']}/{j['n']} | "
            f"{j['extinct']}/{j['n']} | {fmt(j['parent'])} | {fmt(j['herb'])} | {fmt(j['gap'])} | "
            f"{fmt(c1)} | {fmt(c2)} | {fmt(c3) if arm != 'std' else '—'} | "
            f"{fmt(ok) if ok is not None else '（対照）'} |"
        )
    print()
    print("| 条件 | seed | 肉食がいた割合 | 草食の絶滅 | 草食・肉食の数（平均） | 草食→肉食 / 肉食→肉食 / 肉食→草食 | 草食の値 前半→後半 | 肉食の値 前半→後半 | 中間の割合 |")
    print("|---|---|---|---|---|---|---|---|---|")
    for arm in sorted(found):
        for sd in sorted(found[arm]):
            if seeds is not None and sd not in seeds:
                continue
            s = summary(found[arm][sd])
            births = " / ".join(fmt(s[k], 0) for k in ("hc", "cc", "ch"))
            if arm == "std":
                births = f"{fmt(s['born_from_herbivores'], 0)}（確率による）"
            print(
                f"| {arm} | {sd} | {fmt(s['present'])} | {fmt(s['extinct'])} | "
                f"{fmt(s['herbivores'], 0)}・{fmt(s['carnivores'], 1)} | {births} | "
                f"{fmt(s['herb_value_h1'])}→{fmt(s['herb_value_h2'])} | "
                f"{fmt(s['carn_value_h1'])}→{fmt(s['carn_value_h2'])} | {fmt(s['intermediate'])} |"
            )


if __name__ == "__main__":
    main()
