import unittest
from audit_phase075_gate import evaluate, ROOT
class GateTests(unittest.TestCase):
    def test_report_has_separate_technical_and_roadmap_gates(self):
        r=evaluate(); self.assertEqual(r['phase08_decision'],'NO-GO');self.assertEqual(r['static_status'],'PASS')
    def test_not_claiming_unrun_tests(self):
        r=evaluate();self.assertEqual(r['rust_tests'],'NOT_RUN_BY_PYTHON');self.assertEqual(r['web_build'],'NOT_RUN_BY_DEFAULT')
if __name__=='__main__':unittest.main()
