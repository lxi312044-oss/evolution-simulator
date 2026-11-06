import tempfile
import unittest
from pathlib import Path
from analyze_phase0744 import load,window

class Phase0744Tests(unittest.TestCase):
    def test_window_accounting_and_rates(self):
        a=dict(tick=0,toxic_alive=10,nontoxic_alive=10,toxic_fraction=.5,herbivores=4,
               toxin_plant_energy_cost_total=0,toxic_births=0,nontoxic_births=0,
               toxic_age_deaths=0,nontoxic_age_deaths=0,toxic_consumed_deaths=0,nontoxic_consumed_deaths=0)
        b=dict(a,tick=100,toxic_alive=12,nontoxic_alive=9,toxic_fraction=12/21,herbivores=8,
               toxin_plant_energy_cost_total=2,toxic_births=5,nontoxic_births=3,
               toxic_age_deaths=1,nontoxic_age_deaths=2,toxic_consumed_deaths=2,nontoxic_consumed_deaths=2)
        w=window(a,b)
        self.assertEqual(w['toxic']['accounting_residual'],0)
        self.assertEqual(w['nontoxic']['accounting_residual'],0)
        self.assertAlmostEqual(w['toxic']['birth_rate'],5/1100)
    def test_invalid_cumulative_rejected(self):
        headers='seed,tick,toxic_alive,nontoxic_alive,toxic_fraction,herbivores,toxic_births,nontoxic_births,toxic_age_deaths,nontoxic_age_deaths,toxic_consumed_deaths,nontoxic_consumed_deaths,toxin_plant_energy_cost_total'
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'x.csv'
            p.write_text(headers+'\n1,0,1,1,0.5,1,2,0,0,0,0,0,0\n1,100,1,1,0.5,1,1,0,0,0,0,0,0\n')
            with self.assertRaises(ValueError):load(p)
if __name__=='__main__':unittest.main()
