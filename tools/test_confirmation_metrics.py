import copy
import unittest
from confirmation_metrics import lineage_metrics, interaction, primary_summary


class ConfirmationMetricsTests(unittest.TestCase):
    def setUp(self):
        self.args = dict(founders=[[1, 0], [9, 0]], focal=[1, 0], horizon=10,
            births=[dict(tick=2, parent=[1, 0], child=[2, 0]),
                    dict(tick=4, parent=[2, 0], child=[3, 0]),
                    dict(tick=6, parent=[9, 0], child=[4, 0])],
            terminals=[dict(tick=3, id=[1, 0])],
            present=[dict(id=[9, 0], hp=10), dict(id=[2, 0], hp=10),
                     dict(id=[3, 0], hp=0), dict(id=[4, 0], hp=10)], focal_birth_counter=1)

    def test_direct_and_lineage_are_distinct_and_not_added(self):
        r = lineage_metrics(**self.args)
        self.assertEqual((r['direct_births'], r['lineage_births'], r['deeper_births']), (1, 2, 1))
        self.assertEqual(r['born_by_depth'], {1: 1, 2: 1})

    def test_dead_founder_does_not_censor_descendant_births(self):
        r = lineage_metrics(**self.args)
        self.assertFalse(r['focal_living'])
        self.assertEqual(r['lineage_births'], 2)

    def test_hp_zero_presence_is_not_survival(self):
        r = lineage_metrics(**self.args)
        self.assertEqual(r['living_descendants'], 1)
        self.assertEqual(r['present_hp_nonpositive_descendants'], 1)

    def test_extinction_is_retained(self):
        self.args['terminals'] += [dict(id=x['id'], tick=10) for x in self.args['present']]
        self.args['present'] = []
        r = lineage_metrics(**self.args)
        self.assertEqual(r['living_descendants'], 0)
        self.assertEqual(r['lineage_births'], 2)

    def test_generational_entity_id_reuse_is_not_duplicate(self):
        self.args['births'].append(dict(tick=9, parent=[2, 0], child=[1, 1]))
        self.args['present'].append(dict(id=[1, 1], hp=2))
        self.assertEqual(lineage_metrics(**self.args)['lineage_births'], 3)

    def test_reject_missing_or_future_parent(self):
        self.args['births'][0]['parent'] = [999, 0]
        with self.assertRaises(ValueError): lineage_metrics(**self.args)

    def test_reject_duplicate_child(self):
        self.args['births'].append(dict(self.args['births'][-1]))
        with self.assertRaises(ValueError): lineage_metrics(**self.args)

    def test_reject_same_tick_parent_birth(self):
        self.args['births'][1]['tick'] = 2
        with self.assertRaises(ValueError): lineage_metrics(**self.args)

    def test_reject_missing_or_ambiguous_terminal(self):
        for kind in ['missing', 'both']:
            args = copy.deepcopy(self.args)
            if kind == 'missing': args['present'].pop()
            else: args['present'].append(dict(id=[1, 0], hp=3))
            with self.assertRaises(ValueError): lineage_metrics(**args)

    def test_reject_birth_after_removal(self):
        self.args['terminals'].append(dict(tick=4, id=[2, 0]))
        self.args['present'] = [x for x in self.args['present'] if x['id'] != [2, 0]]
        with self.assertRaises(ValueError): lineage_metrics(**self.args)

    def test_reject_counter_mismatch_and_nonfinite_endpoint(self):
        args = copy.deepcopy(self.args)
        args['focal_birth_counter'] = 2
        with self.assertRaises(ValueError): lineage_metrics(**args)
        self.args['present'][0]['hp'] = float('nan')
        with self.assertRaises(ValueError): lineage_metrics(**self.args)

    def test_interaction_preserves_horizon_and_distinguishes_endpoints(self):
        a = dict(horizon=10, direct_births=1, lineage_births=2)
        b = dict(horizon=10, direct_births=1, lineage_births=5)
        self.assertEqual(interaction(a, b, a, a, 'direct_births'), 0)
        self.assertEqual(interaction(a, b, a, a, 'lineage_births'), 3)
        b['horizon'] = 9
        with self.assertRaises(ValueError): interaction(a, b, a, a, 'lineage_births')

    def test_horizon_birth_is_counted(self):
        self.args['births'].append(dict(tick=10, parent=[2, 0], child=[5, 0]))
        self.args['present'].append(dict(id=[5, 0], hp=1))
        self.assertEqual(lineage_metrics(**self.args)['lineage_births'], 3)

    def test_zero_variance_does_not_create_zero_width_evidence(self):
        r = primary_summary([0]*32, [1]*32)
        self.assertIsNone(r['primary']['ci'])
        self.assertFalse(r['positive_signal'])
        self.assertFalse(r['precision_target_met'])

    def test_interaction_alone_does_not_imply_actual_on_benefit(self):
        values = [1, 2]*16
        r = primary_summary(values, [-1, -2]*16)
        self.assertGreater(r['primary']['ci'][0], 0)
        self.assertFalse(r['positive_signal'])

    def test_positive_signal_and_minimum_effect_are_separate(self):
        r = primary_summary([0, 1]*16, [1, 2]*16)
        self.assertTrue(r['positive_signal'])
        self.assertTrue(r['minimum_effect_supported'])
        r = primary_summary([0, 0, 0, 1]*8, [1, 2]*16)
        self.assertTrue(r['positive_signal'])
        self.assertFalse(r['minimum_effect_supported'])

    def test_missing_seed_or_nonfinite_contrast_is_rejected(self):
        for values in [[0]*31, [0]*31+[float('inf')]]:
            with self.assertRaises(ValueError): primary_summary(values, [0]*32)


if __name__ == '__main__':
    unittest.main()
