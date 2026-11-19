import copy
import unittest
from analyze_lineage_cohorts import decompose, group_values


class LineageCohortTests(unittest.TestCase):
    def setUp(self):
        self.args=dict(founders=[[1,0],[9,0]],focal=[1,0],horizon=10,counter=1,width=5,lag=3,
            births=[dict(tick=2,parent=[1,0],child=[2,0]),
                    dict(tick=5,parent=[9,0],child=[50,0]),
                    dict(tick=6,parent=[2,0],child=[3,0]),
                    dict(tick=8,parent=[2,0],child=[4,0])],
            terminals=[dict(tick=3,id=[1,0],trigger='hp_nonpositive'),
                       dict(tick=9,id=[3,0],trigger='hp_nonpositive')],
            present=[dict(id=[9,0],hp=5),dict(id=[50,0],hp=5),
                     dict(id=[2,0],hp=0),dict(id=[4,0],hp=5)])

    def test_partition_and_direct_vs_grandchildren(self):
        r=decompose(**self.args)
        self.assertEqual((r['total']['births'],r['total']['removed'],
                          r['total']['pending_hp0'],r['total']['positive_hp_survivors']),(3,1,1,1))
        self.assertEqual(group_values(r,('generation',-1,1))['births'],1)
        self.assertEqual(group_values(r,('generation',-1,2))['births'],2)

    def test_dead_focal_does_not_censor_descendants(self):
        self.assertEqual(decompose(**self.args)['total']['births'],3)

    def test_unremoved_age_followup_is_not_positive_hp_survival(self):
        r=decompose(**self.args)['cells'][(0,1)]
        self.assertEqual(r['unremoved_at_500'],1)
        self.assertEqual(r['positive_hp_survivors'],0)

    def test_removal_exactly_at_followup_is_failure(self):
        r=decompose(**self.args)['cells'][(1,2)]
        self.assertEqual(r['eligible_500'],1)
        self.assertEqual(r['unremoved_at_500'],0)

    def test_late_present_is_right_censored_and_retained_in_births(self):
        r=decompose(**self.args)['total']
        self.assertEqual((r['late_births'],r['late_censored_present'],r['births']),(1,1,3))

    def test_known_late_removal_is_not_right_censored(self):
        self.args['births'].append(dict(tick=9,parent=[2,0],child=[5,0]))
        self.args['terminals'].append(dict(tick=10,id=[5,0],trigger='hp_nonpositive'))
        r=decompose(**self.args)['total']
        self.assertEqual((r['late_births'],r['late_observed_removals'],r['late_censored_present']),(2,1,1))

    def test_cohort_upper_boundary_included(self):
        self.args['births'][0]['tick']=5
        self.args['terminals'][0]['tick']=6
        r=decompose(**self.args)
        self.assertIn((0,1),r['cells'])
        self.assertEqual(r['cells'][(0,1)]['births'],1)

    def test_horizon_birth_remains_in_last_cohort(self):
        self.args['births'][-1]['tick']=10
        r=decompose(**self.args)
        self.assertEqual(r['cells'][(1,2)]['births'],2)
        self.assertEqual(r['total']['late_censored_present'],1)

    def test_generation_id_reuse_not_collapsed(self):
        self.args['births'].append(dict(tick=9,parent=[2,0],child=[1,1]))
        self.args['present'].append(dict(id=[1,1],hp=3))
        self.assertEqual(decompose(**self.args)['total']['births'],4)

    def test_no_lineage_and_missing_strata_are_zero(self):
        args=dict(founders=[[1,0]],focal=[1,0],births=[],terminals=[],present=[dict(id=[1,0],hp=2)],
                  horizon=10,counter=0,width=5,lag=3)
        r=decompose(**args)
        self.assertEqual(r['cells'],{})
        self.assertEqual(group_values(r,('generation',-1,4))['births'],0)
        self.assertEqual(r['total']['eligible_500'],0)

    def test_observed_time_and_removal_flows(self):
        r=decompose(**self.args)
        self.assertEqual(r['total']['observed_unremoved_ticks'],13)
        self.assertEqual(r['flows'],{(1,2,1,'hp_nonpositive'):1})

    def test_incomplete_endpoint_or_counter_rejected(self):
        for key in ['present','counter']:
            args=copy.deepcopy(self.args)
            if key=='present':args['present'].pop()
            else:args['counter']=2
            with self.assertRaises(ValueError):decompose(**args)

    def test_invalid_partition_parameters_rejected(self):
        for key in ['width','lag']:
            args=copy.deepcopy(self.args);args[key]=0
            with self.assertRaises(ValueError):decompose(**args)


if __name__=='__main__':unittest.main()
