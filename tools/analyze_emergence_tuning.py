"""Issue #11: tuning carnivores born from herbivores (probability and combinations), 40x40.

Plan: https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6998629192

Usage: python tools/analyze_emergence_tuning.py <tuning-result-dir> <emergence-result-dir>

Same P / H / C as tools/analyze_carnivore_emergence.py. The reference v_p002 comes from the
previous emergence results. Web check setting: the candidate if any; otherwise the arm
(including v_p002) meeting H and C with the highest median carnivore presence.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from analyze_carnivore_emergence import summarize  # noqa: E402

TUNING = ["w_p004", "w_p002_b120", "w_p002_def", "w_p004_b120_def"]


def main():
    tuning, emergence = sys.argv[1], sys.argv[2]
    result = {"v_p002": summarize(emergence, "v_p002")}
    result.update({arm: summarize(tuning, arm) for arm in TUNING})
    rank = lambda a: (-result[a]["carnivore_presence_median"], -result[a]["C_cyclic_runs"])  # noqa: E731
    candidates = [a for a in TUNING if result[a]["P"] and result[a]["H"] and result[a]["C"]]
    result["candidates"] = candidates
    if candidates:
        result["web_check"] = {"arm": min(candidates, key=rank), "kind": "candidate"}
    else:
        eligible = [a for a in ["v_p002", *TUNING] if result[a]["H"] and result[a]["C"]]
        result["web_check"] = (
            {"arm": min(eligible, key=rank), "kind": "check_only"} if eligible else None
        )
    print(json.dumps(result, indent=1, ensure_ascii=False, default=lambda x: None))


if __name__ == "__main__":
    main()
