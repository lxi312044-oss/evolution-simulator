import json
import tempfile
import unittest
from pathlib import Path
from audit_phase075_gate import evaluate, REQUIRED


class GateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1])
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        for name, path in REQUIRED.items():
            dest = self.root / path
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_text({"toxins": "pub struct ToxinTypeId", "plants": "produces_toxin: bool",
                             "design": "Phase 08"}.get(name, "fixture"), encoding="utf-8")

    def test_missing_experiment_evidence_fails(self):
        report = evaluate(self.root)
        self.assertEqual(report["static_status"], "FAIL")
        self.assertEqual(report["phase08_decision"], "NO-GO")

    def test_technical_pass_does_not_approve_roadmap(self):
        results = self.root / "results/phase0745"
        results.mkdir(parents=True)
        (results / "REPORT.json").write_text(json.dumps({"status": "PASS"}))
        for seed in range(9):
            (results / f"fixture-seed{seed}.csv").write_text("fixture," * 20)
        report = evaluate(self.root)
        self.assertEqual(report["static_status"], "PASS")
        self.assertEqual(report["phase08_decision"], "NO-GO")

    def test_not_claiming_unrun_tests(self):
        report = evaluate(self.root)
        self.assertEqual(report["rust_tests"], "NOT_RUN_BY_PYTHON")
        self.assertEqual(report["web_build"], "NOT_RUN_BY_DEFAULT")


if __name__ == "__main__":
    unittest.main()
