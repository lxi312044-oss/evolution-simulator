import unittest
from analyze_wasm_lifehistories import individual_window,summarize_window,lifetime_descriptive,LifeHistoryAudit
from run_longterm_resistance import config_for,GERMINATION

def person(birth=1,duration=6000,censored=False,ages=(),r=.6):
 return dict(id=[1,0],birth_tick=birth,resistance=r,resistance_bin=2 if r>=.6 else 1,duration=duration,censored=censored,observed_births=len(ages),lifetime_births=None if censored else len(ages),first_birth_age=ages[0] if ages else None,offspring_ages=list(ages),removal_trigger=None if censored else 'hp_nonpositive',terminal_hp=1 if censored else 0)
class LifeHistoryTests(unittest.TestCase):
 def test_never_breeding_death_is_not_missing_survivor(self):
  w=individual_window(person(duration=100),5000,400);self.assertEqual(w['bred'],0);self.assertEqual(w['removed_before_first'],1);self.assertEqual(w['restricted_first_birth_wait'],5000);self.assertEqual(w['restricted_duration'],100);self.assertEqual(w['mature_duration'],0)
 def test_birth_window_boundaries_and_decomposition(self):
  a=person(duration=11000,censored=True,ages=(400,5000,5001,10000));w=individual_window(a,10000,400);self.assertEqual(w['births'],4);self.assertEqual(w['births_0_5000'],2);self.assertEqual(w['births_after5000'],2);self.assertEqual(w['first_birth_sum'],400);self.assertEqual(w['mature_duration'],9600)
  s=summarize_window([a],10000,400)[0];self.assertAlmostEqual(s['mature_duration']*s['births_per1000_mature_persistence']/1000,s['births_per_individual'])
 def test_late_births_excluded_even_if_already_removed(self):
  rows=[person(birth=45000,duration=5000),person(birth=45001,duration=10)];s=summarize_window(rows,5000,400);self.assertEqual(len(s),1);self.assertEqual(s[0]['birth_window'],8);self.assertEqual(s[0]['individuals'],1)
 def test_censored_lifetime_is_unknown_not_zero(self):
  d=lifetime_descriptive([person(duration=1000,ages=(500,)),person(duration=6000,censored=True,ages=(400,1000))])[-1];self.assertEqual(d['removed_complete_lives'],1);self.assertEqual(d['censored'],1);self.assertEqual(d['confirmed_lifetime_birth_mean_among_removed'],1);self.assertEqual(d['observed_births_in_censored'],2);self.assertIsNone(d['unknown_future_births_in_censored'])
 def test_short_followup_cannot_be_complete_window(self):
  with self.assertRaises(ValueError):individual_window(person(duration=1000,censored=True),5000,400)
 def test_no_breeders_mean_is_null(self):
  w=summarize_window([person(duration=100),person(duration=200,r=.5)],5000,400);self.assertTrue(all(x['first_birth_mean_among_breeders'] is None for x in w))
 def initial(self):
  c=config_for(12345,20,True);c['initial_population']['herbivores']=1;a=LifeHistoryAudit(c,50000);a.accept(dict(kind='initial',config=c,ticks=50000,interval=1000,germination=GERMINATION,founders=[dict(id=[1,0],age=40,resistance=.5)],initial_state={}));return a
 def test_founder_age_includes_preexisting_age(self):
  a=self.initial();a.accept(dict(kind='removal',tick=10,id=[1,0],resistance=.5,generation=0,births=0,age=50,dose=0.,damage=0.,upkeep=0.,hp=0,trigger='hp_nonpositive'));self.assertEqual(len(a.terminals),1)
 def test_offspring_terminal_age_mismatch_rejected(self):
  a=self.initial();a.accept(dict(kind='birth',tick=1,id=[2,0],parent=[1,0],resistance=.5,parent_resistance=.5,generation=1,mutation_attempted=False))
  with self.assertRaisesRegex(ValueError,'terminal age'):a.accept(dict(kind='removal',tick=10,id=[2,0],resistance=.5,generation=1,births=0,age=10,dose=0.,damage=0.,upkeep=0.,hp=0,trigger='hp_nonpositive'))
if __name__=='__main__':unittest.main()
