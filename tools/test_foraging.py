import unittest
from diagnose_foraging import classify,ForagingAudit

def event():
    return dict(id=[1,0],tick=10,visible=1,visible_n=1,visible_ne=1,plant_protected=False,satiety_blocked=False,may_feed=True,action='Forage',desired_step=1.,affordable_step=1.,target=dict(in_range=True,n=1.),feeding=dict(attempted=True,gained_e=1.,eaten_n=1.,available_plant_e=1.,energy_capacity=1.))
class ForagingTests(unittest.TestCase):
    def test_categories(self):
        for patch,category in [({},'positive_income'),({'plant_protected':True},'plant_protected'),({'satiety_blocked':True},'satiety_blocked'),({'target':None},'no_target'),({'target':dict(in_range=False)},'target_out_of_range')]:
            e=event();e.update(patch);self.assertEqual(classify(e),category)
        for patch,category in [({'eaten_n':0},'intake_limited'),({'available_plant_e':0},'plant_no_energy'),({'energy_capacity':0},'reserve_full')]:
            e=event();e['feeding']['gained_e']=0;e['feeding'].update(patch);self.assertEqual(classify(e),category)
    def test_protection_priority_does_not_hide_separate_satiety_flag(self):
        e=event();e.update(plant_protected=True,satiety_blocked=True);self.assertEqual(classify(e),'plant_protected')
    def test_window_and_partition(self):
        a=ForagingAudit([dict(id=[1,0],start=10,end=11)]);a.accept(event());e=event();e.update(tick=11,target=None);e['feeding'].update(gained_e=0,eaten_n=0,attempted=False);a.accept(e)
        self.assertEqual(a.finish()[0]['counts'],dict(no_target=1))
    def test_missing_or_later_income_rejected(self):
        for tick in [11,12]:
            a=ForagingAudit([dict(id=[1,0],start=10,end=11)]);a.accept(event());e=event();e['tick']=tick
            with self.assertRaises(ValueError):a.accept(e)
    def test_nonfinite_rejected(self):
        e=event();e['desired_step']=float('nan')
        with self.assertRaises(ValueError):classify(e)
if __name__=='__main__':unittest.main()
