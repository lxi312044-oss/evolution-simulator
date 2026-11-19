"""Compare current exact common terminal hashes against the archived 0.05 baseline."""
import gzip
import json
from pathlib import Path
import subprocess

from observe_stage4_predator_opportunities import BASE, CONFIG, SEEDS


def main():
    rows = []
    for seed in SEEDS:
        command = ["target/release/examples/stage4_integrated_baseline.exe",
                   str(CONFIG), str(seed), "compatibility"]
        run = json.loads(subprocess.check_output(command, timeout=120))
        with gzip.open(BASE / f"{seed}-005-trace.json.gz", "rt") as stream:
            old = json.load(stream)
        assert run["rows"] == old["rows"] and run["trace"] == old["trace"]
        assert run["first_zero"] == old["first_zero"]
        assert run["common_terminal_state_hash"] == old["terminal_state_hash"], (
            seed, run["common_terminal_state_hash"], old["terminal_state_hash"]
        )
        rows.append({"seed": seed, "old": old["terminal_state_hash"],
                     "current_common": run["common_terminal_state_hash"],
                     "current_full": run["terminal_state_hash"],
                     "recovery_default_all_ticks": True})
    print(json.dumps(rows, indent=2))


if __name__ == "__main__":
    main()
