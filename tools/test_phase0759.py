import csv
import json
import tempfile
import unittest
from pathlib import Path

from analyze_phase0759 import COUNTS, METRICS, ENVIRONMENTS, SEEDS, analyze


class InterventionAnalysisTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parent.parent)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for seed in SEEDS:
            for environment, fraction in ENVIRONMENTS.items():
                for protection in ("on", "off"):
                    run = f"seed{seed}_{environment}_{protection}"
                    config = dict(seed=seed, initial_population=dict(herbivores=80), toxin=dict(
                        enabled=True, resistance_enabled=True, herbivore_multi_toxin_enabled=True,
                        plant_toxin_type_variation_enabled=True, plant_toxin_type_count=2,
                        initial_type0_fraction=fraction, additional_type_resistance_upkeep_multiplier=1.0,
                        multi_type_resistance_upkeep_multiplier=0.5,
                        resistance_effectiveness=0.8 if protection == "on" else 0.0))
                    (self.root / f"{run}.json").write_text(json.dumps(config), encoding="utf-8")
                    rows = []
                    for tick in (0, 100, 150):
                        for ty in (0, 1):
                            row = dict.fromkeys(COUNTS + METRICS, 0)
                            extinct = protection == "off" and tick == 150
                            row.update(seed=seed, tick=tick, toxin_type=ty, herbivores=0 if extinct else 80,
                                       toxic_plants=150, mean_resistance=0 if extinct else 0.5,
                                       deaths_in_window=80 if extinct else 0,
                                       cumulative_herbivore_deaths=80 if extinct else 0)
                            rows.append(row)
                    self.write(run, rows)
        self.run_name = "seed12345_A80_on"

    def write(self, run, rows):
        with (self.root / f"{run}.types.csv").open("w", newline="", encoding="utf-8") as handle:
            writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)

    def mutate_rows(self, change):
        with (self.root / f"{self.run_name}.types.csv").open(newline="", encoding="utf-8") as handle:
            rows = list(csv.DictReader(handle))
        change(rows)
        self.write(self.run_name, rows)

    def check(self):
        return analyze(self.root, 150, 100)

    def test_valid_pairs_and_extinction_missingness(self):
        rows, pairs = self.check()
        self.assertEqual((len(rows), len(pairs)), (24, 12))
        self.assertEqual(rows[2]["first_extinct_sample_tick"], 150)
        self.assertEqual(rows[2]["last_alive_sample_tick"], 100)
        self.assertEqual(rows[2]["final_resistance"], "")
        self.assertEqual(pairs[0]["final_herbivores_on_minus_off"], 80)
        self.assertEqual(pairs[0]["resistance_delta_on_minus_off"], "")

    def test_tick_zero_only_rejected(self):
        self.mutate_rows(lambda rows: rows.__delitem__(slice(2, None)))
        with self.assertRaisesRegex(ValueError, "tick coverage"):
            self.check()

    def test_duplicate_sample_rejected(self):
        self.mutate_rows(lambda rows: rows.append(rows[-1].copy()))
        with self.assertRaisesRegex(ValueError, "tick coverage"):
            self.check()

    def test_missing_type_rejected(self):
        self.mutate_rows(lambda rows: rows.__setitem__(slice(None), [r for r in rows if r["toxin_type"] == "0"]))
        with self.assertRaisesRegex(ValueError, "tick coverage"):
            self.check()

    def test_wrong_seed_rejected(self):
        self.mutate_rows(lambda rows: rows[0].update(seed="999"))
        with self.assertRaisesRegex(ValueError, "seed mismatch"):
            self.check()

    def test_nonfinite_resistance_rejected(self):
        self.mutate_rows(lambda rows: rows[0].update(mean_resistance="nan"))
        with self.assertRaisesRegex(ValueError, "nonfinite"):
            self.check()

    def test_population_balance_rejected(self):
        self.mutate_rows(lambda rows: rows[-1].update(herbivores="79"))
        with self.assertRaisesRegex(ValueError, "population balance"):
            self.check()

    def test_window_counts_rejected(self):
        self.mutate_rows(lambda rows: rows[-1].update(births_in_window="1"))
        with self.assertRaisesRegex(ValueError, "window event"):
            self.check()

    def test_wrong_condition_matrix_rejected(self):
        path = self.root / f"{self.run_name}.types.csv"
        path.rename(self.root / "seed999_A80_on.types.csv")
        with self.assertRaisesRegex(ValueError, "Run matrix"):
            self.check()

    def test_unmatched_configuration_rejected(self):
        path = self.root / f"{self.run_name}.json"
        config = json.loads(path.read_text())
        config["plants"] = {"growth_per_tick": 999}
        path.write_text(json.dumps(config))
        with self.assertRaisesRegex(ValueError, "paired configs differ"):
            self.check()

    def test_wrong_defense_rejected(self):
        path = self.root / f"{self.run_name}.json"
        config = json.loads(path.read_text())
        config["toxin"]["resistance_effectiveness"] = 0
        path.write_text(json.dumps(config))
        with self.assertRaisesRegex(ValueError, "resistance_effectiveness"):
            self.check()

    def test_initial_state_mismatch_rejected(self):
        self.mutate_rows(lambda rows: rows[0].update(toxic_plants="149"))
        with self.assertRaisesRegex(ValueError, "initial states"):
            self.check()


if __name__ == "__main__":
    unittest.main()
