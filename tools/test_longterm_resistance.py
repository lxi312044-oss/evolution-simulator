import copy,unittest
from run_longterm_resistance import Audit,config_for,resistance_bin,GERMINATION
class LongtermTests(unittest.TestCase):
 def test_sizes_preserve_density_and_only_toxin_differs(self):
  small=config_for(12345,20,True);large=config_for(12345,40,True)
  self.assertEqual(large['world']['width'],small['world']['width']*2)
  for k in small['initial_population']:self.assertEqual(large['initial_population'][k],small['initial_population'][k]*4)
  off=config_for(12345,40,False);off['toxin']['enabled']=True;self.assertEqual(off,large)
 def test_float32_boundaries(self):self.assertEqual(resistance_bin(.6),2);self.assertEqual(resistance_bin(.4),0);self.assertEqual(resistance_bin(.5),1)
 def initial(self,a):
  c=config_for(12345,20,True);c['initial_population']['herbivores']=1;a.config=c
  a.accept(dict(kind='initial',config=c,ticks=1000,interval=1000,germination=GERMINATION,founders=[dict(id=[1,0],resistance=.5,age=0)],initial_state={}))
 def test_unlogged_mutation_and_unknown_parent_rejected(self):
  for parent,r in [([2,0],.5),([1,0],.6)]:
   a=Audit({},1000);self.initial(a)
   with self.assertRaises(ValueError):a.accept(dict(kind='birth',tick=1,id=[3,0],parent=parent,parent_resistance=.5,resistance=r,generation=1,mutation_attempted=False))
 def test_nonfinite_and_incomplete_rejected(self):
  a=Audit({},1000)
  with self.assertRaises(ValueError):a.accept(dict(kind='initial',value=float('nan')))
  with self.assertRaises(ValueError):a.finish()

class GenealogyAuditTests(unittest.TestCase):
 def setup_audit(self):
  self.a=Audit({},1000);LongtermTests().initial(self.a)
 def sample(self,tick,empty=False,child=False):
  r=None if empty else .6 if child else .5
  s=dict(kind='snapshot',tick=tick,present=0 if empty else 1,positive=0 if empty else 1,resistance_mean=r,variance=None if empty else 0.,high=int(child),high_fraction=None if empty else float(child),positive_mean=r,positive_high_fraction=None if empty else float(child),max_generation=int(child),births=int(child),removals=int(tick>0),mutation_attempts=int(child),births_by_bin=[0,int(child),0],adult_ticks=[0,0,0],max_n_error=0.,max_e_error=0.)
  for k in ['dose','damage','upkeep','germination_births','protected_ticks','starvation_ticks','capacity_ticks','adult_resistance_sum','parent_resistance_sum','child_resistance_sum']:s[k]=0.
  if child:s['parent_resistance_sum']=.5;s['child_resistance_sum']=.6
  return s
 def test_birth_removal_survivor_and_censoring(self):
  self.setup_audit();a=self.a;a.accept(self.sample(0))
  a.accept(dict(kind='birth',tick=1,id=[2,0],parent=[1,0],resistance=.6,parent_resistance=.5,generation=1,mutation_attempted=True))
  a.accept(dict(kind='removal',tick=2,id=[1,0],resistance=.5,generation=0,births=1,dose=0,damage=0,upkeep=0))
  a.accept(self.sample(1000,child=True));a.accept(dict(kind='survivor',tick=1000,id=[2,0],resistance=.6,generation=1,births=0,hp=1,dose=0,damage=0,upkeep=0))
  a.accept(dict(kind='complete',tick=1000,births=1,removals=1));s=a.finish();self.assertEqual(s['right_censored_offspring'],1);self.assertEqual(s['complete_first5000_cohorts'],[])
 def test_extinction_is_null_not_zero(self):
  self.setup_audit();a=self.a;a.accept(self.sample(0));a.accept(dict(kind='removal',tick=2,id=[1,0],resistance=.5,generation=0,births=0,dose=0,damage=0,upkeep=0))
  bad=self.sample(1000,empty=True);bad['resistance_mean']=0
  with self.assertRaises(ValueError):a.accept(bad)
  # Use a fresh reader because rejected records need not be recoverable.
  self.setup_audit();a=self.a;a.accept(self.sample(0));a.accept(dict(kind='removal',tick=2,id=[1,0],resistance=.5,generation=0,births=0,dose=0,damage=0,upkeep=0));a.accept(self.sample(1000,empty=True));a.accept(dict(kind='complete',tick=1000,births=0,removals=1));self.assertIsNone(a.finish()['final']['resistance_mean'])
 def test_missing_checkpoint_rejected(self):
  self.setup_audit();self.a.accept(self.sample(0));s=self.sample(500)
  with self.assertRaises(ValueError):self.a.accept(s)

if __name__=='__main__':unittest.main()
