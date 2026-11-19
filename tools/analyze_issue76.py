"""Issue #76: summarize mimicry_goal CSVs and judge the goal criteria (P1-P3, M1, M2).

Usage: python tools/analyze_issue76.py RESULTS_DIR [--control ARM=NL_ARM ...] [--diag]
RESULTS_DIR holds <arm>_<seed>.csv files written by the example `mimicry_goal`.

Definitions (Issue #76):
- window: ticks 10000..30000 (points at multiples of 1000); halves 10000..20000 / 20000..30000
- defended share: mean over window points of defended / herbivores
- avoid: mean over window points (blank values skipped)
- attack rate: interval rate (attacks(30000)-attacks(10000)) * 1000 / (exposure difference);
  the last row is used if the run stopped before 30000
- carn_present: share of window points with carnivores > 0
- herbivore extinction: herbivores == 0 at any recorded point
"""
import csv
import glob
import math
import os
import re
import sys
from collections import defaultdict

G = ["defended", "mimic", "other"]
W0, W1, WM = 10000, 30000, 20000


def load(path):
    with open(path, newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))
    for r in rows:
        for k, v in r.items():
            if k == "arm":
                continue
            r[k] = float(v) if v not in ("", None) else None
    return rows


def num(x):
    return x if x is not None and not math.isnan(x) else None


def mean(values):
    v = [x for x in values if x is not None]
    return sum(v) / len(v) if v else None


def at(rows, tick):
    """Row at `tick`, or the last row before it."""
    best = None
    for r in rows:
        if r["tick"] <= tick:
            best = r
    return best


def window(rows, a=W0, b=W1):
    return [r for r in rows if a <= r["tick"] <= b and r["tick"] % 1000 == 0]


def interval(rows, key, a=W0, b=W1):
    ra, rb = at(rows, a), at(rows, b)
    if ra is None or rb is None:
        return None
    return rb[key] - ra[key]


def rate(rows, g, a=W0, b=W1):
    att = interval(rows, f"attacks_{g}", a, b)
    exp = interval(rows, f"exposure_{g}", a, b)
    return att * 1000 / exp if exp else None


def summary(rows):
    w = window(rows)
    s = {}
    s["ticks"] = rows[-1]["tick"]
    s["herb_extinct"] = any(r["herbivores"] == 0 for r in rows)
    s["carn_present"] = mean([1.0 if r["carnivores"] > 0 else 0.0 for r in w])
    s["carn_present_all"] = mean(
        [1.0 if r["carnivores"] > 0 else 0.0 for r in rows if r["tick"] % 1000 == 0]
    )
    s["defended_share"] = mean(
        [r["n_defended"] / r["herbivores"] if r["herbivores"] else None for r in w]
    )
    for g in G:
        s[f"avoid_{g}"] = mean([r[f"avoid_{g}"] for r in w])
        s[f"rate_{g}"] = rate(rows, g)
    for half, (a, b) in {"h1": (W0, WM), "h2": (WM, W1)}.items():
        hw = window(rows, a, b)
        s[f"mimic_share_{half}"] = mean([r["mimic_share"] for r in hw])
        s[f"distance_{half}"] = mean([r["mean_distance"] for r in hw])
    s["herbivores"] = mean([r["herbivores"] for r in w])
    s["carnivores"] = mean([r["carnivores"] for r in w])
    s["toxic_plant_share"] = mean(
        [r["toxic_plants"] / r["plants"] if r["plants"] else None for r in w]
    )
    s["carn_damage"] = rows[-1]["carn_damage"]
    return s


def judge(s, nl=None):
    """Per-seed pass flags for one run (s) and its no-learning control (nl)."""
    out = {}
    d = s["defended_share"]
    out["P1"] = d is not None and 0.10 <= d <= 0.60
    ad, ao = s["avoid_defended"], s["avoid_other"]
    out["P2"] = ad is not None and ad >= 0.20 and (ao is not None and ad >= 2 * ao)
    am = s["avoid_mimic"]
    rm, ro = s["rate_mimic"], s["rate_other"]
    m1 = (
        am is not None
        and ao is not None
        and am - ao >= 0.1
        and rm is not None
        and ro
        and rm <= 0.7 * ro
    )
    if m1 and nl is not None:
        # The learning-free control must lose the difference or keep at most half of it.
        diff_avoid = am - ao
        nl_avoid = (nl["avoid_mimic"] or 0) - (nl["avoid_other"] or 0)
        diff_rate = 1 - rm / ro
        nl_rate = 1 - nl["rate_mimic"] / nl["rate_other"] if nl["rate_other"] and nl["rate_mimic"] is not None else 0
        m1 = nl_avoid <= diff_avoid / 2 and nl_rate <= diff_rate / 2
    out["M1"] = bool(m1)
    up = lambda x: (x["mimic_share_h2"] or 0) - (x["mimic_share_h1"] or 0)
    closer = lambda x: (x["distance_h1"] or 0) - (x["distance_h2"] or 0)
    m2 = up(s) > 0 or closer(s) > 0
    if nl is not None:
        m2 = m2 and (up(s) - up(nl) >= 0.05 or closer(s) - closer(nl) >= 0.01)
    out["M2"] = bool(m2)
    return out


def runs(directory):
    found = defaultdict(dict)
    for path in glob.glob(os.path.join(directory, "*.csv")):
        m = re.match(r"(.+)_(\d+)\.csv$", os.path.basename(path))
        if not m:
            continue
        rows = load(path)
        if rows and "n_defended" in rows[0]:
            found[m.group(1)][int(m.group(2))] = rows
    return found


def fmt(x, nd=2):
    if x is None:
        return "—"
    if isinstance(x, bool):
        return "○" if x else "×"
    return f"{x:.{nd}f}"


def arm_table(found, controls):
    lines = [
        "| 条件 | seed数 | 肉食がいた割合 | 草食の絶滅 | 有毒の割合 | 避けられる割合 有毒/擬態/他 | 攻撃率 有毒/擬態/他 | P1 | P2 | P3 | M1 | M2 |",
        "|---|---|---|---|---|---|---|---|---|---|---|---|",
    ]
    for arm in sorted(found):
        seeds = sorted(found[arm])
        ss = {sd: summary(found[arm][sd]) for sd in seeds}
        nl_arm = controls.get(arm)
        flags = []
        for sd in seeds:
            nl = summary(found[nl_arm][sd]) if nl_arm and sd in found.get(nl_arm, {}) else None
            flags.append(judge(ss[sd], nl))
        n = len(seeds)
        need = math.ceil(n * 3 / 4)
        cnt = lambda k: sum(f[k] for f in flags)
        extinct = sum(ss[sd]["herb_extinct"] for sd in seeds)
        cp = mean([ss[sd]["carn_present"] for sd in seeds])
        p3 = extinct <= n // 4 and cp is not None and cp >= 0.7
        avg = lambda k: mean([ss[sd][k] for sd in seeds])
        lines.append(
            f"| {arm} | {n} | {fmt(cp)} | {extinct} | {fmt(avg('defended_share'))} | "
            + "/".join(fmt(avg(f"avoid_{g}")) for g in G)
            + " | "
            + "/".join(fmt(avg(f"rate_{g}")) for g in G)
            + f" | {cnt('P1')}/{n} | {cnt('P2')}/{n} | {fmt(p3)} | "
            + (f"{cnt('M1')}/{n}" if nl_arm else f"({cnt('M1')}/{n})")
            + " | "
            + (f"{cnt('M2')}/{n}" if nl_arm else f"({cnt('M2')}/{n})")
            + " |"
        )
        _ = need
    return "\n".join(lines)


def seed_table(found):
    lines = [
        "| 条件 | seed | 終了Tick | 草食絶滅 | 肉食がいた割合 | 有毒の割合 | 避け 有毒/擬態/他 | 攻撃率 有毒/擬態/他 | 擬態の割合 前半→後半 | 距離 前半→後半 |",
        "|---|---|---|---|---|---|---|---|---|---|",
    ]
    for arm in sorted(found):
        for sd in sorted(found[arm]):
            s = summary(found[arm][sd])
            lines.append(
                f"| {arm} | {sd} | {int(s['ticks'])} | {fmt(s['herb_extinct'])} | {fmt(s['carn_present'])} | "
                f"{fmt(s['defended_share'])} | "
                + "/".join(fmt(s[f"avoid_{g}"]) for g in G)
                + " | "
                + "/".join(fmt(s[f"rate_{g}"]) for g in G)
                + f" | {fmt(s['mimic_share_h1'])}→{fmt(s['mimic_share_h2'])} | "
                f"{fmt(s['distance_h1'], 3)}→{fmt(s['distance_h2'], 3)} |"
            )
    return "\n".join(lines)


def diag(found):
    out = []
    # Q1: death causes before herbivore extinction.
    out.append("### Q1 草食の絶滅の前 3000 Tick")
    out.append(
        "| 条件 | seed | 絶滅Tick | 出生 | 飢え(E=0)のHP死 | ほかのHP死 | 捕食で倒れた | 寿命 | 植物 始→終 | 肉食 最大 | 肉食の毒死(全期間) |"
    )
    out.append("|---|---|---|---|---|---|---|---|---|---|---|")
    for arm in sorted(found):
        for sd, rows in sorted(found[arm].items()):
            ext = next((r["tick"] for r in rows if r["herbivores"] == 0), None)
            if ext is None:
                continue
            a = ext - 3000
            d = lambda k: interval(rows, k, a, ext)
            ra, rb = at(rows, a), at(rows, ext)
            carn_max = max(r["carnivores"] for r in rows if a <= r["tick"] <= ext)
            out.append(
                f"| {arm} | {sd} | {int(ext)} | {d('herb_births'):.0f} | {d('herb_hp_deaths_zero_e'):.0f} | "
                f"{d('herb_hp_deaths_positive_e'):.0f} | {d('predation_kills'):.0f} | {d('herb_age_deaths'):.0f} | "
                f"{ra['plants']:.0f}→{rb['plants']:.0f} | {carn_max:.0f} | {rows[-1]['carn_toxin_deaths']:.0f} |"
            )
    out.append("")
    out.append("### 全期間の死因（seed合計）と肉食")
    out.append(
        "| 条件 | 草食の出生 | 飢え(E=0)のHP死 | ほかのHP死 | 捕食で倒れた | 寿命 | 肉食の出生(草食から) | 肉食の死亡 | うち毒 | うち飢え |"
    )
    out.append("|---|---|---|---|---|---|---|---|---|---|")
    for arm in sorted(found):
        tot = defaultdict(float)
        for rows in found[arm].values():
            for k in [
                "herb_births",
                "herb_hp_deaths_zero_e",
                "herb_hp_deaths_positive_e",
                "predation_kills",
                "herb_age_deaths",
                "carn_births",
                "carn_from_herbivores",
                "carn_deaths",
                "carn_toxin_deaths",
                "carn_starvation_deaths",
            ]:
                tot[k] += rows[-1][k]
        out.append(
            f"| {arm} | {tot['herb_births']:.0f} | {tot['herb_hp_deaths_zero_e']:.0f} | {tot['herb_hp_deaths_positive_e']:.0f} | "
            f"{tot['predation_kills']:.0f} | {tot['herb_age_deaths']:.0f} | {tot['carn_births']:.0f}({tot['carn_from_herbivores']:.0f}) | "
            f"{tot['carn_deaths']:.0f} | {tot['carn_toxin_deaths']:.0f} | {tot['carn_starvation_deaths']:.0f} |"
        )
    # Q2: stratified attack rates (interval 10000..30000, seeds pooled).
    out.append("")
    out.append("### Q2 区分ごとの攻撃率（10000〜30000 Tick、seed合計、1000個体Tickあたり）")
    out.append(
        "| 条件 | 群 | 全体 | 肉食の近くにいる割合 | 近くにいるときの率 | 近く・大人・エネルギー十分 | 近く・大人・エネルギー不足 | 近く・子ども | 子どもの割合 | エネルギー不足の割合 | 速さ | 体の大きさ | 毒をつくる力 |"
    )
    out.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for arm in sorted(found):
        for gi, g in enumerate(G):
            att = [0.0] * 8
            exp = [0.0] * 8
            prof = defaultdict(list)
            for rows in found[arm].values():
                for s in range(8):
                    att[s] += interval(rows, f"sa_{g}_{s}") or 0
                    exp[s] += interval(rows, f"se_{g}_{s}") or 0
                for r in window(rows):
                    for k in ["speed", "adult_max_hp", "toxin_ability"]:
                        prof[k].append(r[f"{k}_{g}"])
            r_ = lambda idx: (
                sum(att[s] for s in idx) * 1000 / sum(exp[s] for s in idx)
                if sum(exp[s] for s in idx)
                else None
            )
            total_exp = sum(exp)
            near = [s for s in range(8) if s & 4]
            share = lambda idx: sum(exp[s] for s in idx) / total_exp if total_exp else None
            out.append(
                f"| {arm} | {g} | {fmt(r_(range(8)))} | {fmt(share(near))} | {fmt(r_(near))} | "
                f"{fmt(r_([4]))} | {fmt(r_([6]))} | {fmt(r_([5, 7]))} | "
                f"{fmt(share([s for s in range(8) if s & 1]))} | {fmt(share([s for s in range(8) if s & 2]))} | "
                f"{fmt(mean(prof['speed']))} | {fmt(mean(prof['adult_max_hp']), 1)} | {fmt(mean(prof['toxin_ability']))} |"
            )
    # Q3: carnivore memory.
    out.append("")
    out.append("### Q3 肉食の色の記憶（10000〜30000 Tick の平均）")
    out.append(
        "| 条件 | 記憶の色数 | 害のある色数 | 最大の害 | 許せる危険度 | 学んだ色を避ける肉食の割合 | 害のある記憶の押し出し/新しい色 | 色の近い草食の割合 | 肉食の害の合計/移った毒(生成分) |"
    )
    out.append("|---|---|---|---|---|---|---|---|---|")
    for arm in sorted(found):
        acc = defaultdict(list)
        ev = cr = dmg = tp = 0.0
        for rows in found[arm].values():
            for r in window(rows):
                for k in [
                    "mem_mean_entries",
                    "mem_mean_harmful_entries",
                    "mem_mean_max_harm",
                    "mem_mean_tolerance",
                    "mem_avoids_own_harm_color",
                    "color_neighbor_share",
                ]:
                    acc[k].append(r[k])
            ev += interval(rows, "harmful_evictions") or 0
            cr += interval(rows, "memory_entries_created") or 0
            dmg += rows[-1]["carn_damage"]
            tp += rows[-1]["transferred_produced"]
        out.append(
            f"| {arm} | {fmt(mean(acc['mem_mean_entries']), 1)} | {fmt(mean(acc['mem_mean_harmful_entries']), 1)} | "
            f"{fmt(mean(acc['mem_mean_max_harm']), 4)} | {fmt(mean(acc['mem_mean_tolerance']))} | "
            f"{fmt(mean(acc['mem_avoids_own_harm_color']))} | {ev:.0f}/{cr:.0f} | {fmt(mean(acc['color_neighbor_share']))} | "
            f"{dmg:.1f}/{tp:.1f} |"
        )
    return "\n".join(out)


def main():
    args = sys.argv[1:]
    directory = args[0]
    controls = {}
    for a in args[1:]:
        if "=" in a:
            k, v = a.split("=", 1)
            controls[k] = v
    found = runs(directory)
    print(arm_table(found, controls))
    print()
    print(seed_table(found))
    if "--diag" in args:
        print()
        print(diag(found))


if __name__ == "__main__":
    main()
