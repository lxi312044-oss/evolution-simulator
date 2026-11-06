import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from run_metadata import record


class MetadataTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1])
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        for file in ("Cargo.toml", "Cargo.lock", "crates/sim/src/lib.rs",
                     "tools/run_phase0759.ps1", "tools/run_metadata.py", "sim.exe"):
            path = self.root / file
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("fixture")
        self.config = self.root / "config.json"
        self.config.write_text(json.dumps({"seed": 12345}))
        self.output = self.root / "run.metadata.json"
        self.mock = patch("run_metadata.command", return_value=None)
        self.mock.start()
        self.addCleanup(self.mock.stop)

    def record(self, stage):
        return record(stage, self.root, self.config, self.root / "sim.exe", self.output, 100, 10)

    def test_archive_does_not_invent_commit(self):
        data = self.record("begin")
        self.assertIsNone(data["git_commit"])
        self.assertIsNone(data["git_dirty"])
        self.assertIn("crates/sim/src/lib.rs", data["inputs"]["source_sha256"])
        self.assertEqual(self.record("complete")["status"], "completed")

    def test_cannot_overwrite_existing_run(self):
        self.record("begin")
        with self.assertRaises(ValueError):
            self.record("begin")

    def test_changed_inputs_never_mark_complete(self):
        for file in ("config.json", "sim.exe", "crates/sim/src/lib.rs"):
            with self.subTest(file=file):
                if self.output.exists():
                    self.output.unlink()
                self.record("begin")
                path = self.root / file
                old = path.read_bytes()
                path.write_bytes(old + b" ")
                with self.assertRaises(ValueError):
                    self.record("complete")
                self.assertEqual(json.loads(self.output.read_text())["status"], "running")
                path.write_bytes(old)

    def test_completed_run_cannot_be_completed_again(self):
        self.record("begin")
        self.record("complete")
        with self.assertRaises(ValueError):
            self.record("complete")


if __name__ == "__main__":
    unittest.main()
