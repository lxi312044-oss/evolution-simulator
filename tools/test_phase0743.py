import tempfile
import unittest
from pathlib import Path
from analyze_phase0743 import load_csv, summarize

HEADER='seed,tick,toxic_alive,nontoxic_alive,toxic_fraction,toxic_births,nontoxic_births,toxic_age_deaths,nontoxic_age_deaths,toxic_consumed_deaths,nontoxic_consumed_deaths,toxin_plant_energy_cost_total,herbivores\n'
class AnalysisTests(unittest.TestCase):
    def test_plant_time_and_rate(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'test.csv'
            p.write_text(HEADER+'12345,0,5,5,0.5,0,0,0,0,0,0,0,80\n12345,10,5,5,0.5,2,1,0,0,1,0,0.5,80\n')
            s=summarize(load_csv(p))
            self.assertEqual(s['toxic_plant_ticks'],50)
            self.assertAlmostEqual(s['toxic_birth_rate_per_plant_tick'],.04)
            self.assertAlmostEqual(s['toxic_consumed_death_rate_per_plant_tick'],.02)
    def test_reject_decreasing_cumulative(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'bad.csv'
            p.write_text(HEADER+'12345,0,5,5,0.5,2,0,0,0,0,0,0,80\n12345,10,5,5,0.5,1,0,0,0,0,0,0,80\n')
            with self.assertRaises(ValueError): load_csv(p)
if __name__=='__main__':unittest.main()
