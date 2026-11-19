import unittest
from run_wasm_lineage import complete_cohorts
class DescendantTests(unittest.TestCase):
 def test_fixed_window_counts_children_grandchildren_and_censoring(self):
  life={0:dict(founder=True,birth=0,r=.5,children=[100],end=None),1:dict(founder=False,birth=100,r=.6,children=[200,5200],end=None),2:dict(founder=False,birth=200,r=.5,children=[300],end=None),3:dict(founder=False,birth=300,r=.5,children=[],end=400),4:dict(founder=False,birth=5200,r=.6,children=[],end=None)}
  parents={1:0,2:1,3:2,4:1};c=complete_cohorts(life,parents,6000);high=next(x for x in c if x['resistance_bin']==2)
  self.assertEqual(high['individuals'],1);self.assertEqual(high['offspring_first5000'],1);self.assertEqual(high['descendants_first5000'],2);self.assertEqual(high['grandchildren_first5000'],1)
 def test_closed_stratum_and_late_birth_exclusion(self):
  life={0:dict(founder=True,birth=0,r=.5,children=[5000,5001],end=None),1:dict(founder=False,birth=5000,r=.6,children=[],end=None),2:dict(founder=False,birth=5001,r=.6,children=[],end=None)}
  rows=complete_cohorts(life,{1:0,2:0},10000);self.assertEqual(len(rows),1);self.assertEqual(rows[0]['birth_window'],0);self.assertEqual(rows[0]['individuals'],1)
if __name__=='__main__':unittest.main()
