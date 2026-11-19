"""Run the six fixed Stage 4 inherited-color conditions, with no result overwrite."""
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    binary = Path("target/release/examples") / (
        "stage4_herbivore_learning.exe" if os.name == "nt" else "stage4_herbivore_learning"
    )
    config = Path("config/stage4-inherited-plant-colors.json")
    output = Path("results/stage4-inherited-plant-colors")
    limit = 10 * 1024**2
    used = 0
    output.mkdir()
    manifest = {
        "base_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "binary_sha256": sha256(binary),
        "config_sha256": sha256(config),
        "conditions": [],
    }
    for seed in (12345, 12346, 12347):
        for learning in ("on", "off"):
            row = {"seed": seed, "learning": learning}
            try:
                completed = subprocess.run(
                    [str(binary), str(seed), f"inherited-{learning}"],
                    capture_output=True, check=True, timeout=120,
                )
                result = json.loads(completed.stdout)
                if (result["seed"], result["learning"], result["ticks"]) != (
                    seed, learning == "on", 6000
                ):
                    raise ValueError("unexpected run identity")
                packed = gzip.compress(completed.stdout, mtime=0)
                if used + len(packed) > limit - 16384:
                    raise ValueError("persistent output limit")
                path = output / f"{seed}-{learning}.json.gz"
                with path.open("xb") as stream:
                    stream.write(packed)
                used += len(packed)
                row.update(file=path.name, bytes=len(packed), sha256=sha256(path),
                           seconds=result["seconds"])
            except (subprocess.SubprocessError, ValueError) as error:
                stderr = getattr(error, "stderr", None) or b""
                row.update(error=str(error), stderr=stderr.decode(errors="replace")[-1000:])
            manifest["conditions"].append(row)
            print(json.dumps(row), flush=True)
    with (output / "manifest.json").open("x", encoding="utf-8") as stream:
        json.dump(manifest, stream, indent=2)
    if any("error" in row for row in manifest["conditions"]):
        raise SystemExit("fixed comparison incomplete")


if __name__ == "__main__":
    main()
