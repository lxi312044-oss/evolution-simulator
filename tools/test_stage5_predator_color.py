"""Artificial data only. No simulator, experiment seed input, or fitted model selection."""
import itertools
import math
import unittest

from analyze_stage5_predator_color import analyze

SEEDS = tuple(range(1, 129))
COLORS = tuple(itertools.product(range(50, 151, 25), (50, 75, 125, 150), (50, 75, 125, 150)))


def artificial(effect):
    for seed in SEEDS:
        for index, (r, g, b) in enumerate(COLORS):
            yield dict(seed=seed, index=index, generation=0, r8=r, g8=g, b8=b,
                       on_births=4 + effect(seed, r, g, b), off_births=4)


class PrimaryAnalysisTest(unittest.TestCase):
    def test_zero(self):
        result = analyze(artificial(lambda *_: 0), SEEDS)
        self.assertEqual(result["decision"], "not_supported")
        for interval in result["intervals"].values():
            self.assertEqual(interval["mean"], 0.0)

    def test_linear_scale_and_interval(self):
        result = analyze(artificial(lambda s, r, *_: ((r - 100) // 25) * (1 + s % 2)), SEEDS)
        red = result["intervals"]["r8"]
        self.assertEqual(result["decision"], "supported")
        self.assertAlmostEqual(red["mean"], 1.53)
        self.assertAlmostEqual(red["seed_sd"], 0.51 * math.sqrt(128 / 127))
        self.assertAlmostEqual(result["critical_t"], 2.426104917, places=8)
        width = result["critical_t"] * red["seed_sd"] / math.sqrt(128)
        self.assertAlmostEqual(red["lower"], 1.53 - width)
        self.assertAlmostEqual(red["upper"], 1.53 + width)
        for color in ("g8", "b8"):
            self.assertAlmostEqual(result["intervals"][color]["mean"], 0.0)

    def test_local_effect_has_zero_linear_projection(self):
        rows = list(artificial(lambda _, r, g, b: 6 if r == 100 and g in (75, 125) and b in (75, 125) else 0))
        self.assertEqual(sum(row["on_births"] - row["off_births"] for row in rows), 24 * 128)
        result = analyze(rows, SEEDS)
        self.assertEqual(result["decision"], "not_supported")
        for interval in result["intervals"].values():
            self.assertAlmostEqual(interval["mean"], 0.0)

    def test_rank_failure_forces_uncertain(self):
        rows = list(artificial(lambda _seed, r, *_: (r - 100) // 25))
        for row in rows:
            if row["seed"] == 1:
                row["g8"] = row["r8"]
        result = analyze(rows, SEEDS)
        self.assertEqual(result["decision"], "uncertain")
        self.assertTrue(result["intervals_descriptive_only"])
        self.assertEqual(result["invalid_seeds"], {1: "rank-deficient RGB regression"})


if __name__ == "__main__":
    unittest.main()
