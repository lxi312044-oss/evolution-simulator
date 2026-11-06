import csv, tempfile, unittest
from pathlib import Path
from analyze_phase0745 import analyze
class TestPhase0745(unittest.TestCase):
 def test_summary(self):
  with tempfile.TemporaryDirectory() as d:
   p=Path(d)/'standard-seed12345.csv'
   fields=['seed','tick','trait','alive','energy_mean','energy_min','energy_max','seed_attempts','seed_success','seed_resource_blocked','resource_ready']
   with p.open('w',newline='') as h:
    w=csv.DictWriter(h,fieldnames=fields);w.writeheader()
    for tick in (0,100):
     for trait in ('toxic','nontoxic'):
      w.writerow(dict(seed=12345,tick=tick,trait=trait,alive=10,energy_mean=1,energy_min=0,energy_max=2,seed_attempts=tick//10,seed_success=tick//20,seed_resource_blocked=0,resource_ready=5))
   result=analyze(Path(d),100)
   self.assertEqual(result['runs'],1)
   self.assertEqual(result['traits'][0]['parent_seed_success'],5)
 def test_missing_ending_fails(self):
  with tempfile.TemporaryDirectory() as d:
   p=Path(d)/'zero-seed1.csv';p.write_text('trait,tick\ntoxic,0\n',encoding='utf-8')
   with self.assertRaises(ValueError):analyze(Path(d),100)
if __name__=='__main__':unittest.main()
