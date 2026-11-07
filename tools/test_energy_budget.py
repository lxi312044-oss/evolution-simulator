import copy
import unittest
from diagnose_energy_budget import EnergyAudit

class EnergyAuditTests(unittest.TestCase):
    def audit(self,hp=None,removed=True):
        return EnergyAudit([dict(id=[1,0],birth_tick=1,removal=dict(tick=2) if removed else None)], [dict(id=[1,0],first_hp0=hp)])
    def event(self,n,phase,b,a,d,tick=1,hs=0):
        return dict(seq=n,id=[1,0],tick=tick,phase=phase,before_e=b,after_e=a,delta_e=d,health_seq=hs)
    def test_known_budget_and_closure(self):
        a=self.audit()
        rows=[('birth',0,10,10),('feeding',10,15,5),('basal',15,13,-2),('movement',13,12,-1),('resistance_upkeep',12,11,-1),('offspring_transfer',11,8,-3),('reproduction_work',8,7,-1),('predation_bite',7,5,-2),('end_tick',5,5,0),('predation_lethal_meal',5,2,-3),('removal_heat',2,0,-2),('removal',0,0,0)]
        for i,r in enumerate(rows,1):a.accept(self.event(i,*r,tick=1 if i<=9 else 2))
        self.assertEqual(a.finish()[0]['balance'],0)
        self.assertEqual(a.max_residual,0)
    def test_missing_transfer_rejected(self):
        a=self.audit();a.accept(self.event(1,'birth',0,10,10))
        with self.assertRaisesRegex(ValueError,'unaccounted'):a.accept(self.event(2,'basal',9,8,-1))
    def test_overstated_payment_rejected(self):
        a=self.audit();a.accept(self.event(1,'birth',0,10,10))
        with self.assertRaisesRegex(ValueError,'unaccounted'):a.accept(self.event(2,'basal',10,9,-2))
    def test_replenishment_and_lethal_order(self):
        a=self.audit(dict(tick=1,seq=5));rows=[('birth',0,1,1,0),('basal',1,0,-1,3),('feeding',0,2,2,4),('predation_lethal_meal',2,0,-2,5)]
        for i,(p,b,v,d,h) in enumerate(rows,1):a.accept(self.event(i,p,b,v,d,hs=h))
        s=a.states[(1,0)];self.assertEqual(s['depletion_episodes'],2);self.assertTrue(s['first_depletion']['before_first_hp0'])
        b=self.audit(dict(tick=1,seq=5));b.accept(self.event(1,'birth',0,2,2));b.accept(self.event(2,'predation_lethal_meal',2,0,-2,hs=5))
        self.assertFalse(b.states[(1,0)]['first_depletion']['before_first_hp0'])
    def test_missing_tick_rejected(self):
        a=self.audit();a.accept(self.event(1,'birth',0,10,10))
        with self.assertRaisesRegex(ValueError,'missing per-tick'):a.accept(self.event(2,'end_tick',10,10,0,tick=2))
    def test_missing_birth_rejected(self):
        with self.assertRaisesRegex(ValueError,'missing birth'):self.audit().accept(self.event(1,'basal',10,9,-1))
    def test_nonfinite_unknown_and_wrong_sign(self):
        for phase,after,delta in [('basal',float('nan'),-1),('unknown',9,-1),('feeding',9,-1)]:
            a=self.audit();a.accept(self.event(1,'birth',0,10,10))
            with self.assertRaises(ValueError):a.accept(self.event(2,phase,10,after,delta))
if __name__=='__main__':unittest.main()
