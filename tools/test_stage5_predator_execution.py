"""Artificial lineage completeness checks for the fixed confirmation runner."""
import copy
import unittest

from run_stage5_predator_color import audit


class LineageTest(unittest.TestCase):
    def setUp(self):
        self.run = {
            "seed": 1, "on": True, "tick": 500, "audit_points": [0, 500],
            "max_population": 82, "max_n_error": 0., "max_e_error": 0., "max_toxin_error": 0.,
            "founders": [{"id": [i, 0]} for i in range(80)],
            "births": [
                {"tick": 1, "id": [80, 0], "parent": [0, 0], "rgb8": [51, 50, 49]},
                {"tick": 3, "id": [0, 1], "parent": [80, 0], "rgb8": [51, 50, 49]},
            ],
            "removals": [{"tick": 2, "id": [0, 0], "reason": "hp_nonpositive"}],
            "survivors": [[i, 0] for i in range(1, 81)] + [[0, 1]],
            "samples": [
                {"tick": 0, "births": 0, "removals": 0, "herbivores": 80},
                {"tick": 500, "births": 2, "removals": 1, "herbivores": 81},
            ],
        }

    def test_descendants_after_founder_death_and_id_reuse(self):
        _, counts, depth = audit(self.run, 1, "on", 500)
        self.assertEqual(counts[0, 0], 2)
        self.assertEqual(counts[1, 0], 0)
        self.assertEqual(depth, {1: 1, 2: 1})

    def test_removed_parent_is_rejected(self):
        broken = copy.deepcopy(self.run)
        broken["births"][1]["parent"] = [0, 0]
        with self.assertRaisesRegex(ValueError, "invalid parent"):
            audit(broken, 1, "on", 500)


if __name__ == "__main__":
    unittest.main()
