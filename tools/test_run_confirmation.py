import copy
import unittest
from run_confirmation import LogValidator, validate_pairs, bootstrap


CASE = (23451, 'off', 0, 'low', 'off')
R = 0.20000000298023224


def fixture(defense=0):
    config = dict(seed=23451, toxin=dict(enabled=False, resistance_enabled=True,
        herbivore_multi_toxin_enabled=True, plant_toxin_type_variation_enabled=True,
        plant_toxin_type_count=2, initial_type0_fraction=1,
        plant_toxin_type_mutation_probability=0, resistance_mutation_probability=0,
        initial_resistance=0.5, resistance_effectiveness=defense,
        multi_type_resistance_upkeep_multiplier=0.5, additional_type_resistance_upkeep_multiplier=1))
    initial = dict(kind='initial', seed=23451, environment='off', toxin_type=0,
        resistance=R, defense=defense, ticks=100, interval=100, focal_id=[1,0],
        initial=dict(hp=10,births=0), effective_config=config,
        initial_background_sha256='same_background', founders=[[1,0],[9,0]],
        founder_traits=[dict(id=[1,0],resistance=[R,0.5]),dict(id=[9,0],resistance=[0.5,0.5])])
    birth = dict(kind='birth', tick=25, parent=[1,0], child=[2,0], resistance=[R,0.5], depth=1)
    snapshot = dict(kind='snapshot', tick=100, present=[dict(id=[1,0],hp=10),
        dict(id=[9,0],hp=10),dict(id=[2,0],hp=0)],focal_birth_counter=1,
        metrics=dict(horizon=100,direct_births=1,lineage_births=1,deeper_births=0,
            living_descendants=0,present_hp_nonpositive_descendants=1,removed_descendants=0,
            born_by_depth={'1':1},living_by_depth={},focal_living=True,focal_present_hp_nonpositive=False))
    legacy = {key:initial[key] for key in ['seed','environment','toxin_type','resistance','defense',
                                         'ticks','interval','focal_id','initial','effective_config']}
    legacy.update(completed_tick=100,max_n_error=0,max_e_error=0,outcome=dict(hp=10,births=1))
    complete = dict(kind='complete',legacy_result=legacy,final_rng='unchanged',birth_event_count=1,
                    removal_event_count=0,final_population=3,plant_type_counts=[2,0])
    return [initial,birth,snapshot,complete]


def validate(rows, case=CASE):
    v=LogValidator(case,100)
    for row in rows:v.accept(row)
    return v.finish()[0]


class LoggerValidationTests(unittest.TestCase):
    def test_independent_genealogy_matches_pending_hp0_endpoint(self):
        result=validate(fixture())
        self.assertEqual(result['endpoints']['100']['present_hp_nonpositive_descendants'],1)
        self.assertEqual(result['endpoints']['100']['living_descendants'],0)

    def test_defense_metadata_is_not_a_dynamics_difference(self):
        a,b=validate(fixture()),validate(fixture(0.8),(*CASE[:-1],'on'))
        self.assertEqual(a['trajectory_sha256'],b['trajectory_sha256'])
        validate_pairs({CASE:a,(*CASE[:-1],'on'):b})

    def test_actual_toxin_off_dynamics_difference_is_rejected(self):
        rows=fixture(0.8)
        rows[2]['present'][0]['hp']=9
        a,b=validate(fixture()),validate(rows,(*CASE[:-1],'on'))
        with self.assertRaises(ValueError):validate_pairs({CASE:a,(*CASE[:-1],'on'):b})

    def test_truncated_or_duplicate_completion_rejected(self):
        for rows in [fixture()[:-1],fixture()+[copy.deepcopy(fixture()[-1])]]:
            with self.assertRaises(ValueError):validate(rows)

    def test_missing_endpoint_or_snapshot_rejected(self):
        rows=fixture();rows[2]['present'].pop()
        with self.assertRaises(ValueError):validate(rows)
        rows=fixture();rows.pop(2)
        with self.assertRaises(ValueError):validate(rows)

    def test_unexpected_mutation_or_birth_depth_rejected(self):
        for key,value in [('resistance',[0.5,0.5]),('depth',2)]:
            rows=fixture();rows[1][key]=value
            with self.assertRaises(ValueError):validate(rows)

    def test_wrong_counter_config_or_event_count_rejected(self):
        for kind in ['counter','config','count']:
            rows=fixture()
            if kind=='counter':rows[2]['focal_birth_counter']=2
            elif kind=='config':rows[0]['effective_config']['toxin']['resistance_mutation_probability']=0.1
            else:rows[-1]['birth_event_count']=2
            with self.assertRaises(ValueError):validate(rows)

    def test_nonfinite_hp_or_contaminated_type_rejected(self):
        rows=fixture();rows[2]['present'][0]['hp']=float('nan')
        with self.assertRaises(ValueError):validate(rows)
        rows=fixture();rows[-1]['plant_type_counts']=[1,1]
        with self.assertRaises(ValueError):validate(rows)

    def test_background_mismatch_rejected(self):
        a,b=validate(fixture()),validate(fixture(0.8),(*CASE[:-1],'on'))
        b['initial_background_sha256']='different'
        with self.assertRaises(ValueError):validate_pairs({CASE:a,(*CASE[:-1],'on'):b})

    def test_bootstrap_preserves_seed_vectors_and_zero_variance(self):
        vectors=[dict(a=i%3,b=2*(i%3),zero=0) for i in range(32)]
        result=bootstrap(vectors)
        self.assertIsNone(result['zero'])
        self.assertEqual(result['b'],[2*x for x in result['a']])
        self.assertEqual(result,bootstrap(vectors))


if __name__=='__main__':unittest.main()
