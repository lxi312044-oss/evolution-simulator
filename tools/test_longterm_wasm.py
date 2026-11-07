import copy,unittest
from unittest.mock import patch
from run_longterm_wasm import validate,config_for
class WebLongtermTests(unittest.TestCase):
 def row(self):
  c=config_for(12345,20,False);n=c['initial_population']['herbivores'];return dict(ticks=50000,config=c,input_sha256='hash',wasm_sha256='hash',js_sha256='hash',initial_n=0.,initial_e=0.,samples=[dict(tick=t,present=n,positive=n,high=0,resistance_mean=.5,high_fraction=0.,dose=0.,damage=0.,n_error=0.,e_error=0.,total_n=0.,total_e=0.,sunlight_in=0.,heat_out=0.) for t in range(0,50001,1000)])
 def setUp(self):
  m=patch('run_longterm_wasm.sha256',return_value='hash');m.start();self.addCleanup(m.stop)
 def test_valid_paired_runtime_record(self):validate(self.row(),(12345,20,False))
 def test_rejects_bad_runtime_missing_tick_and_off_exposure(self):
  for change in [lambda r:r.update(wasm_sha256='wrong'),lambda r:r['samples'].pop(),lambda r:r['samples'][-1].update(dose=1.),lambda r:r['samples'][-1].update(resistance_mean=float('nan'))]:
   r=self.row();change(r)
   with self.assertRaises(ValueError):validate(r,(12345,20,False))
 def test_existing_conservation_tolerance_is_not_relaxed(self):
  r=self.row();r['samples'][-1].update(total_e=.002,e_error=.002)
  with self.assertRaises(ValueError):validate(r,(12345,20,False))
 def test_extinction_requires_null(self):
  r=self.row();r['samples'][-1].update(present=0,positive=0,resistance_mean=None,high_fraction=None);validate(r,(12345,20,False));r['samples'][-1]['resistance_mean']=0.
  with self.assertRaises(ValueError):validate(r,(12345,20,False))
if __name__=='__main__':unittest.main()
