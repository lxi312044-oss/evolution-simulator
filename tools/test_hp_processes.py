import unittest
from diagnose_hp_processes import HealthAudit, rank_cases


def event(seq,tick,phase,before,after,damage=0):
    return dict(seq=seq,tick=tick,phase=phase,id=[2,0],hp_before=before,hp_after=after,
        crossed_hp0=before is not None and before>0 and after<=0,
        recovered_positive=before is not None and before<=0 and after>0,
        reserve_e=10,reserve_n=1,stomach_e=1,stomach_n=1,body_n=1,toxin_damage=0,upkeep_paid=0,
        details=dict(damage=damage))


def member(removal=None):
    return dict(id=[2,0],parent=[1,0],birth_tick=1,depth=1,removal=removal)


class HpProcessTests(unittest.TestCase):
    def test_selection_ties_use_seed_order_and_not_survivor_filtering_afterwards(self):
        rows=[dict(seed=s,birth_difference=1,survival_difference=-1,window_removal_difference=d)
              for s,d in [(4,2),(3,2),(2,1)]]
        selected,fallback=rank_cases(rows)
        self.assertFalse(fallback)
        self.assertEqual([x['seed'] for x in selected],[3,4])

    def test_selection_fallback_is_fixed(self):
        rows=[dict(seed=4,birth_difference=0,survival_difference=0,window_removal_difference=2)]
        selected,fallback=rank_cases(rows)
        self.assertTrue(fallback)
        self.assertEqual(selected,rows)

    def test_first_crossing_keeps_prior_toxin_and_starvation_contributions(self):
        removal=dict(tick=3,hp=-1,trigger='hp_nonpositive')
        audit=HealthAudit([member(removal)])
        for e in [event(1,1,'birth',None,10),event(2,2,'toxin',10,7,3),
                  event(3,2,'starvation',7,4,3),event(4,2,'predation',4,0,5),
                  event(5,3,'toxin',0,-1,1),event(6,3,'pre_removal',None,-1)]:audit.accept(e)
        result=audit.finish([])[0]
        self.assertEqual(result['first_hp0']['phase'],'predation')
        self.assertEqual(result['first_prior_hp_damage'],dict(toxin=3,starvation=3))
        self.assertEqual(result['hp_damage_through_first_hp0'],dict(toxin=3,starvation=3,predation=4))
        self.assertEqual(result['nominal_hp_damage']['predation'],5)
        self.assertEqual(result['crossing_count'],1)

    def test_recovery_does_not_erase_first_crossing(self):
        audit=HealthAudit([member(dict(tick=4,hp=-1,trigger='hp_nonpositive'))])
        for e in [event(1,1,'birth',None,2),event(2,2,'toxin',2,0,2),
                  event(3,3,'recovery',0,1),event(4,4,'starvation',1,-1,2),event(5,4,'pre_removal',None,-1)]:audit.accept(e)
        result=audit.finish([])[0]
        self.assertEqual(result['crossing_count'],2)
        self.assertEqual(result['positive_recovery_count'],1)
        self.assertEqual(result['first_hp0']['phase'],'toxin')

    def test_age_removal_without_hp0_is_not_a_damage_cause(self):
        audit=HealthAudit([member(dict(tick=3,hp=2,trigger='age_limit'))])
        audit.accept(event(1,1,'birth',None,2));audit.accept(event(2,3,'pre_removal',None,2))
        self.assertIsNone(audit.finish([])[0]['first_hp0'])

    def test_unremoved_target_requires_exact_final_hp(self):
        audit=HealthAudit([member()]);audit.accept(event(1,1,'birth',None,2))
        self.assertIsNone(audit.finish([dict(id=[2,0],hp=2)])[0]['removal_tick'])
        with self.assertRaises(ValueError):audit.finish([dict(id=[2,0],hp=1)])

    def test_continuity_missing_birth_and_wrong_flags_fail(self):
        for which in ['gap','missing_birth','flags']:
            audit=HealthAudit([member()])
            rows=[event(1,1,'birth',None,2),event(2,2,'toxin',2,1,1)]
            if which=='gap':rows[1]['hp_before']=3
            elif which=='missing_birth':rows=rows[1:];rows[0]['seq']=1
            else:rows[1]['crossed_hp0']=True
            with self.assertRaises(ValueError):
                for row in rows:audit.accept(row)

    def test_null_and_nonfinite_health_state_fail(self):
        for value in [None,float('nan')]:
            audit=HealthAudit([member()]);row=event(1,1,'birth',None,2);row['reserve_e']=value
            with self.assertRaises(ValueError):audit.accept(row)

    def test_upkeep_tipping_zero_is_context_not_automatic_cause(self):
        audit=HealthAudit([member(dict(tick=3,hp=-1,trigger='hp_nonpositive'))])
        audit.accept(event(1,1,'birth',None,1))
        row=event(2,3,'resistance_upkeep',None,1);row['reserve_e']=0;row['details']={'reserve_before':0.1,'paid_e':0.1}
        audit.accept(row)
        row=event(3,3,'starvation',1,-1,2);row['reserve_e']=0;audit.accept(row)
        row=event(4,3,'pre_removal',None,-1);row['reserve_e']=0;audit.accept(row)
        result=audit.finish([])[0]
        self.assertTrue(result['first_hp0_upkeep_tipped_zero'])
        self.assertEqual(result['first_hp0']['phase'],'starvation')


if __name__=='__main__':unittest.main()
