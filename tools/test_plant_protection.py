import copy,io,json,unittest
from unittest.mock import patch
from compare_plant_protection import validate
class ProtectionComparisonTests(unittest.TestCase):
    def setUp(self):
        c=dict(herbivores=dict(plant_protection_enabled=True),toxin=dict(plant_toxin_type_mutation_probability=0,resistance_mutation_probability=0,initial_type0_fraction=1,resistance_effectiveness=.8))
        self.row=dict(seed=34501,ticks=6000,interval=100,resistance=.2,protection=True,germination=False,effective_config=c,samples=[dict(tick=t) for t in range(100,6001,100)],max_n_error=0.,max_e_error=0.,min_plants=199,zero_plant_ticks=0,germination_births=0,germination_parameters=None)
        original=json.dumps(dict(effective_config=copy.deepcopy(c)))
        mock=patch('compare_plant_protection.gzip.open',side_effect=lambda *a,**k:io.StringIO(original));mock.start();self.addCleanup(mock.stop)
    def test_valid_control(self):validate(self.row,34501,'low',True,False)
    def test_invalid_evidence_rejected(self):
        for change in [lambda r:r.update(seed=1),lambda r:r['samples'].pop(),lambda r:r.update(max_n_error=.0011),lambda r:r.update(max_e_error=float('nan')),lambda r:r['effective_config']['toxin'].update(dose_per_nutrient=1.)]:
            r=copy.deepcopy(self.row);change(r)
            with self.assertRaises(ValueError):validate(r,34501,'low',True,False)
    def test_no_germination_births_in_off_arm(self):
        r=copy.deepcopy(self.row);r['germination_births']=1
        with self.assertRaises(ValueError):validate(r,34501,'low',True,False)
if __name__=='__main__':unittest.main()
