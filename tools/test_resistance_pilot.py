import copy
import unittest
from run_resistance_pilot import matrix, validate_rows, effects, summarize_interactions


def fixture():
    rows = []
    for case in matrix([1, 2]):
        seed, env, ty, level, defense = case
        r = 0.2 if level == "low" else 0.8
        sample = dict(tick=0, alive=True, age=500, hp=50, births=0, dose_a=0, dose_b=0,
                      damage=0, prevented=0, cost=0, trigger="none",
                      resistance_a=r if ty == 0 else 0.5, resistance_b=r if ty == 1 else 0.5)
        outcome = dict(sample, tick=100)
        outcome["births"] = int(env != "off" and ((env == "A") == (ty == 0)) and level == "high" and defense == "on")
        t = dict(enabled=env!="off", initial_type0_fraction=0 if env=="B" else 1,
                 plant_toxin_type_count=2, plant_toxin_type_mutation_probability=0,
                 resistance_mutation_probability=0, initial_resistance=0.5, resistance_enabled=True,
                 herbivore_multi_toxin_enabled=True, multi_type_resistance_upkeep_multiplier=0.5,
                 additional_type_resistance_upkeep_multiplier=1,
                 resistance_effectiveness=0.8 if defense=="on" else 0)
        rows.append(dict(case=list(case), seed=seed, environment=env, toxin_type=ty, resistance=r,
                         defense=t["resistance_effectiveness"], ticks=100, interval=100, completed_tick=100,
                         effective_config=dict(seed=seed, toxin=t), initial=sample, outcome=outcome,
                         checkpoints=[sample, outcome], initial_background_sha256=f"{seed}-{env}",
                         final_ecology=dict(n=1), max_n_error=0, max_e_error=0))
    return rows


class PilotTests(unittest.TestCase):
    def test_full_factorial_and_interaction(self):
        cases = validate_rows(fixture(), [1, 2], 100, 100)
        contrasts, interactions = effects(cases, [1, 2])
        self.assertEqual((len(cases), len(contrasts), len(interactions)), (48, 24, 12))
        matching = [r for r in interactions if r["environment"] == "A" and r["toxin_type"] == 0]
        self.assertTrue(all(r["births_interaction"] == 1 for r in matching))

    def test_missing_case_rejected(self):
        with self.assertRaisesRegex(ValueError, "Missing"):
            validate_rows(fixture()[:-1], [1, 2], 100, 100)

    def test_background_difference_rejected(self):
        rows = fixture()
        rows[0]["initial_background_sha256"] = "changed"
        with self.assertRaisesRegex(ValueError, "backgrounds"):
            validate_rows(rows, [1, 2], 100, 100)

    def test_nonfinite_damage_rejected(self):
        rows = fixture()
        rows[0]["outcome"]["damage"] = float("nan")
        with self.assertRaisesRegex(ValueError, "Nonfinite"):
            validate_rows(rows, [1, 2], 100, 100)

    def test_wrong_resistance_rejected(self):
        rows = fixture()
        rows[0]["outcome"]["resistance_b"] = 0.99
        with self.assertRaisesRegex(ValueError, "resistance changed"):
            validate_rows(rows, [1, 2], 100, 100)

    def test_wrong_horizon_rejected(self):
        rows = fixture()
        rows[0]["completed_tick"] = 99
        with self.assertRaisesRegex(ValueError, "horizon"):
            validate_rows(rows, [1, 2], 100, 100)

    def test_type_contamination_rejected(self):
        rows = fixture()
        rows[0]["outcome"]["dose_b"] = 1
        with self.assertRaisesRegex(ValueError, "contamination"):
            validate_rows(rows, [1, 2], 100, 100)

    def test_toxin_off_equivalence_rejected(self):
        rows = copy.deepcopy(fixture())
        row = next(r for r in rows if r["environment"]=="off" and r["defense"]==0.8)
        row["final_ecology"]["n"] = 2
        with self.assertRaisesRegex(ValueError, "defense toggle"):
            validate_rows(rows, [1, 2], 100, 100)

    def test_zero_sample_variance_does_not_invent_precision(self):
        _, interactions = effects(validate_rows(fixture(), [1, 2], 100, 100), [1, 2])
        summary = summarize_interactions(interactions)
        self.assertTrue(all(r["uncertainty"]=="not_estimable_zero_sample_variation" for r in summary))
        self.assertTrue(all(r["exploratory_ci_low"]=="" and r["rough_normal_n_for_0_2_births"]=="" for r in summary))

    def test_missing_checkpoint_rejected(self):
        rows = fixture()
        rows[0]["checkpoints"] = rows[0]["checkpoints"][:1]
        with self.assertRaisesRegex(ValueError, "checkpoint"):
            validate_rows(rows, [1, 2], 100, 100)


if __name__ == "__main__":
    unittest.main()
