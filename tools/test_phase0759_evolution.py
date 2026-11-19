import csv
import json
from pathlib import Path
import tempfile
import unittest
from analyze_phase0759_evolution import audit_run, summarize, compare_generations, generation_components, fitness_associations


class EvolutionAssessmentTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1])
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.run = "seed12345_equal_on"
        (self.root / f"{self.run}.json").write_text(json.dumps(dict(seed=12345, initial_population=dict(herbivores=1))))
        self.births = []
        self.samples = []
        self.outcomes = []
        # EntityId reuse generations 99/0/100 must never become biological depth.
        for ty in (0, 1):
            child_r = 0.6 if ty == 0 else 0.4
            self.births += [
                dict(seed=12345, tick=1, toxin_type=ty, parent_id=1, parent_generation=99,
                     child_id=2, child_generation=0, parent_resistance=0.5, child_resistance=child_r,
                     delta=child_r-0.5, mutation_observed="true"),
                dict(seed=12345, tick=2, toxin_type=ty, parent_id=2, parent_generation=0,
                     child_id=1, child_generation=100, parent_resistance=child_r, child_resistance=child_r,
                     delta=0, mutation_observed="false")]
            self.samples += [
                self.sample(ty, 0, "alive", 1, 99, 0.5, 0, 0, 0),
                self.sample(ty, 1, "death", 1, 99, 0.5, 0, 0, 0),
                self.sample(ty, 2, "alive", 2, 0, child_r, 2 if ty == 0 else 0, 1, 1),
                self.sample(ty, 2, "alive", 1, 100, child_r, 0, 0, 0),
                self.sample(ty, 4, "alive", 2, 0, child_r, 3 if ty == 0 else 0, 3, 1),
                self.sample(ty, 4, "alive", 1, 100, child_r, 1 if ty == 0 else 0, 2, 0)]
            self.outcomes.append({k: v for k, v in self.samples[-5].items() if k != "lifetime_exposure"})
        self.types = [dict(seed=12345, tick=t, toxin_type=ty, herbivores=n,
                           cumulative_herbivore_births=b, cumulative_herbivore_deaths=d)
                      for t, n, b, d in ((0, 1, 0, 0), (2, 2, 2, 1), (4, 2, 2, 1)) for ty in (0, 1)]
        self.flush()

    def sample(self, ty, tick, event, index, generation, resistance, dose, age, births):
        return dict(seed=12345, tick=tick, event=event, toxin_type=ty, id_index=index,
                    id_generation=generation, resistance=resistance, lifetime_exposure=dose,
                    age=age, successful_births=births, load=0)

    def write(self, suffix, rows):
        with (self.root / f"{self.run}.{suffix}.csv").open("w", newline="", encoding="utf-8") as h:
            writer = csv.DictWriter(h, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)

    def flush(self):
        for suffix, rows in (("births", self.births), ("individuals", self.samples),
                             ("outcomes", self.outcomes), ("types", self.types)):
            self.write(suffix, rows)

    def audit(self):
        self.flush()
        return audit_run(self.root, self.run, 4, 2)

    def test_lineage_depth_and_same_tick_missing_birth(self):
        rows = self.audit()
        self.assertEqual(sorted({r["generation_depth"] for r in rows}), [0, 1, 2])
        founders = [r for r in rows if r["founder"]]
        self.assertTrue(all(r["birth_tick"] == "" and r["unobserved_death_tick_births"] == 1 for r in founders))
        self.assertEqual(len(rows), 6)

    def test_missing_parent_rejected(self):
        self.births[0]["parent_id"] = 999
        self.births[2]["parent_id"] = 999
        with self.assertRaisesRegex(ValueError, "missing parent"):
            self.audit()

    def test_missing_type_rejected(self):
        self.births.pop()
        with self.assertRaisesRegex(ValueError, "missing paired type"):
            self.audit()

    def test_missing_terminal_rejected(self):
        self.samples = [r for r in self.samples if not (r["tick"] == 4 and r["id_index"] == 1)]
        with self.assertRaises(ValueError):
            self.audit()

    def test_duplicate_snapshot_rejected(self):
        self.samples.append(self.samples[0].copy())
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.audit()

    def test_nonfinite_exposure_rejected(self):
        self.samples[0]["lifetime_exposure"] = float("nan")
        with self.assertRaisesRegex(ValueError, "nonfinite"):
            self.audit()

    def test_changed_inherited_resistance_rejected(self):
        self.samples[-1]["resistance"] = 0.9
        with self.assertRaisesRegex(ValueError, "resistance change"):
            self.audit()

    def test_population_counts_rejected(self):
        self.types[-2]["cumulative_herbivore_births"] = 1
        with self.assertRaisesRegex(ValueError, "birth event count"):
            self.audit()

    def test_outcome_mismatch_rejected(self):
        self.outcomes[0]["load"] = 9
        with self.assertRaisesRegex(ValueError, "outcome/snapshot"):
            self.audit()

    def test_partial_cohort_comparison_is_missing(self):
        rows = self.audit()
        summary = summarize(rows, ("seed", "environment", "protection", "toxin_type", "generation_depth"))
        pairs = compare_generations(summary)
        self.assertTrue(all(r["mean_resistance_on_minus_off"] == "" and not r["off_present"] for r in pairs))

    def test_types_have_independent_exposure(self):
        rows = self.audit()
        self.assertTrue(any(r["exposure_terminal_lower_bound"] > 0 for r in rows if r["toxin_type"] == 0))
        self.assertTrue(all(r["exposure_terminal_lower_bound"] == 0 for r in rows if r["toxin_type"] == 1))

    def test_decomposition_separates_parent_representation_and_mutation(self):
        rows = self.audit()
        generations = summarize(rows, ("run", "seed", "environment", "protection", "toxin_type", "generation_depth"))
        parts = generation_components(generations)
        first = next(r for r in parts if r["toxin_type"] == 0 and r["generation_depth"] == 1)
        self.assertAlmostEqual(first["parental_representation_component"], 0)
        self.assertAlmostEqual(first["transmission_component"], 0.1)
        self.assertAlmostEqual(first["arithmetic_residual"], 0)

    def test_zero_variance_does_not_fabricate_correlation(self):
        associations = fitness_associations(self.audit())
        self.assertTrue(all(r["descriptive_correlation"] == "" for r in associations))

    def test_positive_offspring_covariance(self):
        rows = [dict(run="test", seed=1, environment="equal", protection="on", toxin_type=0,
                     birth_cohort="0", founder=False, resistance_at_birth_or_initial=res,
                     logged_children=n, exposure_terminal_lower_bound=1, outcome="death")
                for res, n in ((0.2, 0), (0.8, 2))]
        result = fitness_associations(rows)[0]
        self.assertAlmostEqual(result["descriptive_correlation"], 1)
        self.assertAlmostEqual(result["descriptive_reproductive_differential"], 0.3)

    def make_current_terminal(self):
        for row in self.samples:
            death = row["event"] == "death"
            row.update(observation_phase="pre_removal_current_tick" if death else "end_tick",
                       removal_trigger="age_limit" if death else "none", hp=1,
                       toxin_damage_total=0.5, prevented_damage_total=0.1, resistance_cost_total=0.2)
            if death:
                row["successful_births"] = 1
        for row in self.outcomes:
            row.update(observation_phase="pre_removal_current_tick", removal_trigger="age_limit",
                       hp=1, successful_births=1)

    def test_current_terminal_includes_same_tick_birth(self):
        self.make_current_terminal()
        rows = self.audit()
        terminal = [r for r in rows if r["founder"]]
        self.assertTrue(all(not r["exposure_is_lower_bound"] for r in terminal))
        self.assertTrue(all(r["unobserved_death_tick_births"] == 0 for r in terminal))
        self.assertTrue(all(r["terminal_successful_births_lower_bound"] == 1 for r in terminal))

    def test_inconsistent_current_terminal_phase_rejected(self):
        self.make_current_terminal()
        self.samples[1]["observation_phase"] = "previous_tick"
        with self.assertRaisesRegex(ValueError, "phase mismatch"):
            self.audit()

    def test_nonfinite_current_physiology_rejected(self):
        self.make_current_terminal()
        self.samples[1]["toxin_damage_total"] = float("nan")
        with self.assertRaisesRegex(ValueError, "invalid physiology"):
            self.audit()

    def test_legacy_death_is_still_a_lower_bound(self):
        rows = self.audit()
        self.assertTrue(all(r["exposure_is_lower_bound"] for r in rows if r["outcome"] == "death"))

    def test_signed_f32_rounding_residual_is_preserved(self):
        self.make_current_terminal()
        for row in self.samples:
            row["prevented_damage_total"] = -1e-8
        rows = self.audit()
        self.assertTrue(all(r["prevented_damage_all_types"] == -1e-8 for r in rows))

    def test_prevented_damage_outside_rounding_bound_rejected(self):
        self.make_current_terminal()
        self.samples[1]["prevented_damage_total"] = -0.01
        with self.assertRaisesRegex(ValueError, "rounding bound"):
            self.audit()

    def test_shared_physiology_must_match_across_types(self):
        self.make_current_terminal()
        self.samples[0]["toxin_damage_total"] = 0.9
        with self.assertRaisesRegex(ValueError, "shared physiology"):
            self.audit()


if __name__ == "__main__":
    unittest.main()
